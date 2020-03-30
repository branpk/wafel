from typing import *

from wafel.util import align_up


DataSpec = Any


def spec_populate_sizes_and_alignment(spec: DataSpec, populate_offsets = False) -> None:
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
      if populate_offsets:
        offset = 0
        for field in type_['fields'].values():
          offset = align_up(offset, field['type']['align'])
          field['offset'] = offset
          offset += field['type']['size']
      type_['align'] = max((f['type']['align'] for f in type_['fields'].values()), default=1)
      type_['size'] = align_up(
        max((f['offset'] + f['type']['size'] for f in type_['fields'].values()), default=0),
        type_['align'],
      )

    elif type_['kind'] == 'union':
      if any('size' not in f['type'] for f in type_['fields'].values()):
        type_queue.append(type_)
        continue
      if populate_offsets:
        for field in type_['fields'].values():
          field['offset'] = 0
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


def spec_get_concrete_type(spec: DataSpec, type_: dict) -> dict:
  while type_['kind'] == 'symbol':
    type_ = spec['types'][type_['namespace']][type_['name']]
  return type_


__all__ = [
  'DataSpec',
  'spec_populate_sizes_and_alignment',
  'spec_get_concrete_type',
]
