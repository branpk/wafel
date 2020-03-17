from typing import *
import io
import dataclasses
from dataclasses import dataclass
import textwrap
import os
import json
import hashlib
import ctypes

import pefile
from elftools.dwarf.dwarfinfo import DWARFInfo, DwarfConfig, DebugSectionDescriptor
from elftools.dwarf.compileunit import CompileUnit
from elftools.dwarf.die import DIE

from wafel.core.game_lib import DataSpec, GameLib
from wafel.util import *
import wafel.config as config
from wafel.loading import Loading, in_progress, load_child, load_prefix, load_scale


# TODO: Non-windows
# TODO: 32 bit
# TODO: Object fields and macros


CURRENT_SPEC_FORMAT_VERSION = 1


@dataclass(frozen=True)
class Section:
  raw_address: int
  raw_size: int
  virtual_address: int
  virtual_size: int
  data: bytes


def parse_lib(path: str) -> Dict[str, Section]:
  dll = pefile.PE(path)

  sections: Dict[str, Section] = {}
  for section in dll.sections:
    name = section.Name.strip(b'\x00').decode('utf-8')
    if name.startswith('/'):
      name = dll.strings[int(name[1:])]
    sections[name] = Section(
      raw_address = section.get_file_offset(),
      raw_size = section.SizeOfRawData,
      virtual_address = section.VirtualAddress,
      virtual_size = section.Misc_VirtualSize,
      data = section.get_data(),
    )

  return sections


def read_dwarf_info(sections: Dict[str, Section]) -> List[CompileUnit]:
  def get_section(name: str) -> DebugSectionDescriptor:
    section = sections.get(name)
    if section is None:
      return None
    return DebugSectionDescriptor(
      stream = io.BytesIO(section.data),
      name = name,
      global_offset = section.raw_address,
      size = section.raw_size,
      address = section.virtual_address,
    )

  # TODO: 32 bit support
  dwarf = DWARFInfo(
    DwarfConfig(little_endian=True, machine_arch='x64', default_address_size=8),
    get_section('.debug_info'),
    get_section('.debug_aranges'),
    get_section('.debug_abbrev'),
    get_section('.debug_frame'),
    get_section('.eh_frame'),
    get_section('.debug_str'),
    get_section('.debug_loc'),
    get_section('.debug_ranges'),
    get_section('.debug_line'),
    get_section('.debug_pubtypes'),
    get_section('.debug_pubnames'),
  )

  compilation_units = []
  try:
    for unit in dwarf.iter_CUs():
      compilation_units.append(unit)
  except:
    # TODO: Check that offset got far enough
    pass

  return compilation_units


def attr_value(node: DIE, attr_name: str) -> Optional[object]:
  attr = node.attributes.get(attr_name)
  return None if attr is None else attr.value


def attr_str(node: DIE, attr_name: str) -> Optional[str]:
  attr = node.attributes.get(attr_name)
  return None if attr is None else attr.value.decode('utf-8')


def print_die(die: DIE, recurse=True, indent=''):
  ref_offset = die.offset - die.cu.cu_offset
  log.debug(textwrap.indent(f'[{ref_offset}] {die}', indent))
  if recurse:
    for child in die.iter_children():
      print_die(child, recurse, indent + '>   ')


PRIMITIVES: Dict[str, str] = {
  'char': 's8',
  'long long unsigned int': 'u64',
  'long long int': 's64',
  'short unsigned int': 'u16',
  'int': 's32',
  'long int': 's32',
  'unsigned int': 'u32',
  'long unsigned int': 'u32',
  'unsigned char': 'u8',
  'double': 'f64',
  'float': 'f32',
  'long double': 'f128',
  'signed char': 's8',
  'short int': 's16',
  '_Bool': 's32',
}

VOID = {
  'kind': 'primitive',
  'name': 'void',
  'size': 0,
  'align': 1,
}

ANON_FIELD_NAME = '__anon'


def unique_name(used_names: Set[str], name: str) -> str:
  if name not in used_names:
    return name
  k = 1
  while f'{name}_{k}' in used_names:
    k += 1
  return f'{name}_{k}'


def extract_definitions(spec: DataSpec, root: DIE) -> None:
  types_by_offset: Dict[int, dict] = {}
  placeholders: List[dict] = []

  def at_offset(offset):
    if offset is None:
      return None
    placeholders.append({ '__placeholder': True, 'offset': offset })
    return placeholders[-1]

  assert root.tag == 'DW_TAG_compile_unit'

  for node in root.iter_children():
    offset = node.offset - node.cu.cu_offset
    name = attr_str(node, 'DW_AT_name')
    type_ = at_offset(attr_value(node, 'DW_AT_type'))
    size = attr_value(node, 'DW_AT_byte_size')

    if node.tag == 'DW_TAG_base_type':
      assert name is not None
      types_by_offset[offset] = {
        'kind': 'primitive',
        'name': PRIMITIVES[name],
        'size': size,
        'align': size,
      }

    elif node.tag == 'DW_TAG_const_type':
      types_by_offset[offset] = type_ or VOID

    elif node.tag == 'DW_TAG_volatile_type':
      types_by_offset[offset] = type_ or VOID

    elif node.tag == 'DW_TAG_typedef':
      if name in PRIMITIVES.values():
        types_by_offset[offset] = type_
      else:
        spec['types']['typedef'][name] = type_
        types_by_offset[offset] = {
          'kind': 'symbol',
          'namespace': 'typedef',
          'name': name,
        }

    elif node.tag == 'DW_TAG_pointer_type':
      types_by_offset[offset] = {
        'kind': 'pointer',
        'base': type_ or VOID,
        'size': size,
        'align': size,
      }

    elif node.tag in ['DW_TAG_structure_type', 'DW_TAG_union_type']:
      kind = 'struct' if node.tag == 'DW_TAG_structure_type' else 'union'

      field_names: Set[str] = set()
      for child in node.iter_children():
        field_name = attr_str(child, 'DW_AT_name')
        if field_name is not None:
          field_names.add(field_name)

      fields = {}
      for child in node.iter_children():
        assert child.tag == 'DW_TAG_member'
        field_name = attr_str(child, 'DW_AT_name') or unique_name(field_names, ANON_FIELD_NAME)
        field_names.add(field_name)
        fields[field_name] = {
          'type': at_offset(child.attributes['DW_AT_type'].value),
          'offset': 0 if kind == 'union' else child.attributes['DW_AT_data_member_location'].value,
        }

      type_defn = {
        'kind': kind,
        'fields': fields,
      }
      if name is None:
        types_by_offset[offset] = type_defn
      else:
        spec['types'][kind][name] = type_defn
        types_by_offset[offset] = {
          'kind': 'symbol',
          'namespace': kind,
          'name': name,
        }

    elif node.tag == 'DW_TAG_enumeration_type':
      types_by_offset[offset] = type_
      for child in node.iter_children():
        assert child.tag == 'DW_TAG_enumerator'
        spec['constants'][attr_str(child, 'DW_AT_name')] = {
          'type': type_,
          'source': 'enum',
          'enum_name': name,
          'value': child.attributes['DW_AT_const_value'].value,
        }

    elif node.tag == 'DW_TAG_array_type':
      child = list(node.iter_children())[0]
      assert child.tag == 'DW_TAG_subrange_type'
      assert 'DW_AT_lower_bound' not in child.attributes
      if 'DW_AT_upper_bound' in child.attributes:
        length = child.attributes['DW_AT_upper_bound'].value + 1
      else:
        length = None
      types_by_offset[offset] = {
        'kind': 'array',
        'base': type_,
        'length': length,
      }

    elif node.tag == 'DW_TAG_subroutine_type':
      assert name is None
      params = []
      for child in node.iter_children():
        assert child.tag == 'DW_TAG_formal_parameter'
        assert 'DW_AT_name' not in child.attributes
        params.append({
          'name': None,
          'type': at_offset(child.attributes['DW_AT_type'].value),
        })
      types_by_offset[offset] = {
        'kind': 'function',
        'return': type_ or VOID,
        'params': params,
        'variadic': None, # TODO
        'size': None,
        'align': None,
      }

    elif node.tag == 'DW_TAG_variable':
      if name is not None:
        spec['globals'][name] = { 'type': type_ }

    elif node.tag == 'DW_TAG_subprogram':
      if name is not None:
        params = []
        for child in node.iter_children():
          if child.tag == 'DW_TAG_formal_parameter':
            params.append({
              'name': attr_str(child, 'DW_AT_name'),
              'type': at_offset(child.attributes['DW_AT_type'].value),
            })
        spec['globals'][name] = {
          'type': {
            'kind': 'function',
            'return': type_ or VOID,
            'params': params,
            'variadic': None, # TODO
            'size': None,
            'align': None,
          },
        }

    else:
      print_die(node)
      raise NotImplementedError(f'Unhandled: {node.tag}')

  while len(placeholders) > 0:
    placeholder = placeholders.pop(0)
    type_ = types_by_offset.get(placeholder['offset'])
    if type_ is not None and '__placeholder' not in type_:
      placeholder.clear()
      placeholder.update(type_)
    else:
      placeholders.append(placeholder)


def extract_all_definitions(
  spec: DataSpec,
  compilation_units: List[CompileUnit],
) -> Loading[None]:
  for i, unit in enumerate(compilation_units):
    root = unit.get_top_DIE()
    filename = assert_not_none(attr_str(root, 'DW_AT_name'))
    yield in_progress(i / len(compilation_units), ' - ' + filename)
    extract_definitions(spec, root)
  yield in_progress(1.0)


def populate_sizes_and_alignment(spec: DataSpec) -> None:
  type_queue = []
  def get_all_types(node):
    if isinstance(node.get('kind'), str):
      type_queue.append(node)
    for child in node.values():
      if isinstance(child, dict):
        get_all_types(child)
  get_all_types(spec)

  while len(type_queue) > 0:
    type_ = type_queue.pop(0)
    if 'size' in type_ and 'align' in type_:
      continue

    if type_['kind'] == 'struct':
      if any('size' not in f['type'] for f in type_['fields'].values()):
        type_queue.append(type_)
        continue
      type_['align'] = max((f['type']['align'] for f in type_['fields'].values()), default=1)
      type_['size'] = align_up(
        max((f['offset'] + f['type']['size'] for f in type_['fields'].values()), default=0),
        type_['align'],
      )

    elif type_['kind'] == 'union':
      if any('size' not in f['type'] for f in type_['fields'].values()):
        type_queue.append(type_)
        continue
      type_['align'] = max(f['type']['align'] for f in type_['fields'].values())
      type_['size'] = align_up(
        max(f['type']['size'] for f in type_['fields'].values()),
        type_['align'],
      )

    elif type_['kind'] == 'array':
      if 'size' not in type_['base']:
        type_queue.append(type_)
        continue
      type_['stride'] = align_up(type_['base']['size'], type_['base']['align'])
      type_['align'] = type_['base']['align']
      type_['size'] = None if type_['length'] is None else type_['length'] * type_['stride']

    elif type_['kind'] == 'symbol':
      concrete_type = spec['types'][type_['namespace']][type_['name']]
      if 'size' not in concrete_type:
        type_queue.append(type_)
        continue
      type_['size'] = concrete_type['size']
      type_['align'] = concrete_type['align']

    else:
      raise NotImplementedError(type_['kind'])


EXAMPLE_SM64_SOURCE_FILE = 'src/game/level_update.c'

def find_sm64_compile_dir(compilation_units: List[CompileUnit]) -> str:
  for unit in compilation_units:
    root = unit.get_top_DIE()
    if attr_str(root, 'DW_AT_name') == EXAMPLE_SM64_SOURCE_FILE:
      return assert_not_none(attr_str(root, 'DW_AT_comp_dir'))
  raise Exception(f'Could not find {EXAMPLE_SM64_SOURCE_FILE} in library')

def find_sm64_units(compilation_units: List[CompileUnit]) -> Loading[List[CompileUnit]]:
  sm64_compile_dir = find_sm64_compile_dir(compilation_units)
  sm64_units = []
  for i, unit in enumerate(compilation_units):
    yield in_progress(i / len(compilation_units))
    if attr_str(unit.get_top_DIE(), 'DW_AT_comp_dir') == sm64_compile_dir:
      sm64_units.append(unit)
  return sm64_units

def hash_file(path: str) -> str:
  with open(path, 'rb') as f:
    contents = f.read()
  return hashlib.md5(contents).hexdigest()


def extract_data_spec(path: str) -> Loading[DataSpec]:
  yield in_progress(0.0)
  sections = parse_lib(path)

  yield in_progress(0.2)
  compilation_units = read_dwarf_info(sections)
  compilation_units = yield from load_scale(0.2, 0.25, find_sm64_units(compilation_units))

  spec: DataSpec = {
    'format_version': CURRENT_SPEC_FORMAT_VERSION,
    'library_hash': hash_file(path),
    'sections': {
      name: {
        'raw_address': section.raw_address,
        'raw_size': section.raw_size,
        'virtual_address': section.virtual_address,
        'virtual_size': section.virtual_size,
      } for name, section in sections.items()
    },
    'types': {
      'struct': {},
      'union': {},
      'typedef': {},
    },
    'globals': {},
    'constants': {},
  }

  yield from load_scale(0.25, 0.9, extract_all_definitions(spec, compilation_units))

  populate_sizes_and_alignment(spec)

  yield in_progress(1.0)
  return spec


def extract_data_spec_cached(path: str) -> Loading[DataSpec]:
  rel_path = os.path.relpath(path, config.lib_directory)
  cache_key = f'libsm64_spec_{rel_path}'
  spec = config.cache_get(cache_key)

  library_hash = hash_file(path)
  if spec is not None and \
      spec['format_version'] == CURRENT_SPEC_FORMAT_VERSION and \
      spec['library_hash'] == library_hash:
    log.info(f'Cache hit for {os.path.basename(path)}')
    return spec

  spec = yield from extract_data_spec(path)

  lib_name = os.path.splitext(os.path.basename(path))[0]
  config.cache_put(cache_key, spec, f'{lib_name}*.json')

  return spec


def load_libsm64(game_version: str) -> Loading[GameLib]:
  filename = f'sm64_{game_version}.dll'
  log.info('Loading', filename)
  path = os.path.join(config.lib_directory, 'libsm64', filename)

  dll = ctypes.cdll.LoadLibrary(path)

  status = f'Loading {filename}'
  yield in_progress(0.0, status)

  spec = yield from load_child(
    0.0, 0.95, status, extract_data_spec_cached(path),
  )

  # TODO: Hacks until macros/object fields are implemented
  with open(os.path.join(config.assets_directory, 'hack_constants.json'), 'r') as f:
    spec['constants'].update(json.load(f))
  with open(os.path.join(config.assets_directory, 'hack_object_fields.json'), 'r') as f:
    spec['extra'] = {}
    spec['extra']['object_fields'] = json.load(f)

  yield in_progress(1.0, status)
  log.info('Done loading', filename)
  return GameLib(spec, dll)
