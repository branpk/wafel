from typing import *
from ctypes import cdll
import json
import gc
import os
import weakref

import ext_modules.util as c_util

import wafel.config as config
from wafel.core import Game, Timeline, load_dll_game, Controller, Address, DataPath, \
  AccessibleMemory, SlotState
from wafel.variable import Variable, VariableAccessor
from wafel.edit import Edits
from wafel.object_type import ObjectType
from wafel.loading import Loading
from wafel.util import *
from wafel.script import Scripts, ScriptController
from wafel.data_variables import DataVariables
from wafel.range_edit import RangeEditAccessor


class EditController(Controller):
  def __init__(self, data_variables: DataVariables, edits: Edits) -> None:
    super().__init__()
    self.data_variables = data_variables
    self.edits = edits
    self.edits.on_edit(self.weak_notify)

  def apply(self, state: SlotState) -> None:
    for edit in self.edits.get_edits(state.frame):
      self.data_variables.set_raw(state, edit.variable, edit.value)


class DataVariableAccessor(VariableAccessor):
  def __init__(
    self,
    timeline: Timeline,
    data_variables: DataVariables,
    addr_to_symbol: Dict[Address, str],
    edits: Edits,
  ) -> None:
    self.timeline = timeline
    self.data_variables = data_variables
    self.addr_to_symbol = addr_to_symbol
    self.edits = edits

  def get(self, variable: Variable) -> object:
    frame = variable.args['frame']

    # TODO: Possibly move to DataVariables?
    object_slot: Optional[int] = variable.args.get('object')
    if object_slot is not None:
      object_type: Optional[ObjectType] = variable.args.get('object_type')
      if object_type is not None and self.get_object_type(frame, object_slot) != object_type:
        return None

    surface_index: Optional[int] = variable.args.get('surface')
    if surface_index is not None:
      num_surfaces = dcast(int, self.timeline.get(frame, 'gSurfacesAllocated'))
      if surface_index >= num_surfaces:
        return None

    return self.data_variables.get(self.timeline[frame], variable)

  def get_object_type(self, frame: int, object_slot: int) -> Optional[ObjectType]:
    active = self.get(Variable('obj-active-flags-active', frame=frame, object=object_slot))
    if not active:
      return None
    behavior_addr = self.get(Variable('obj-behavior-ptr', frame=frame, object=object_slot))
    assert isinstance(behavior_addr, Address)
    return ObjectType(behavior_addr, self.addr_to_symbol[behavior_addr])

  def set(self, variable: Variable, value: object) -> None:
    frame: int = variable.args['frame']

    object_slot: Optional[int] = variable.args.get('object')
    if object_slot is not None:
      object_type: Optional[ObjectType] = variable.args.get('object_type')
      if object_type is not None and self.get_object_type(frame, object_slot) != object_type:
        return

    surface_index: Optional[int] = variable.args.get('surface')
    if surface_index is not None:
      num_surfaces = dcast(int, self.timeline.get(frame, 'gSurfacesAllocated'))
      if surface_index >= num_surfaces:
        return

    self.edits.edit(variable, value)

  def reset(self, variable: Variable) -> None:
    self.edits.reset(variable)

  def edited(self, variable: Variable) -> bool:
    return self.edits.edited(variable)


class ScriptVariableAccessor(VariableAccessor):
  def __init__(self, scripts: Scripts) -> None:
    self.scripts = scripts

  def get(self, variable: Variable) -> object:
    frame: int = variable.args['frame']
    return self.scripts.get(frame).source

  def set(self, variable: Variable, value: object) -> None:
    frame: int = variable.args['frame']
    self.scripts.set_frame_source(frame, dcast(str, value))

  def reset(self, variable: Variable) -> None:
    frame: int = variable.args['frame']
    self.scripts.reset_frame(frame)

  def edited(self, variable: Variable) -> bool:
    frame: int = variable.args['frame']
    return self.scripts.is_edited(frame)


class Model:

  def __init__(self) -> None:
    self.game_version: str
    self.timeline: Timeline
    self.rotational_camera_yaw = 0
    self.input_up_yaw: Optional[int] = None

  def load(self, game_version: str, edits: Edits) -> Loading[None]:
    yield from self._load_game(game_version)
    self._set_edits(edits)

  def change_version(self, game_version: str) -> Loading[None]:
    yield from self.load(game_version, self.edits)

  def _load_game(self, game_version: str) -> Loading[None]:
    if hasattr(self, 'game_version') and self.game_version == game_version:
      return
    self.game_version = game_version

    self.game = yield from load_dll_game(
      os.path.join(config.lib_directory, 'libsm64', 'sm64_' + game_version + '.dll'),
      'sm64_init',
      'sm64_update',
    )

    # TODO: Hacks until macros/object fields are implemented
    with open(os.path.join(config.assets_directory, 'hack_constants.json'), 'r') as f:
      self.game.memory.data_spec['constants'].update(json.load(f))
    with open(os.path.join(config.assets_directory, 'hack_object_fields.json'), 'r') as f:
      object_fields = json.load(f)
      object_struct = self.game.memory.data_spec['types']['struct']['Object']
      for name, field in object_fields.items():
        object_struct['fields'][name] = {
          'offset': object_struct['fields']['rawData']['offset'] + field['offset'],
          'type': field['type'],
        }

    memory = self.game.memory
    assert isinstance(memory, AccessibleMemory)
    c_util.init(lambda name: memory.address_to_location(self.game.base_slot, memory.symbol(name)))

    self.data_variables = DataVariables(self.game)

    self.action_names: Dict[int, str] = {}
    for constant_name, constant in self.game.memory.data_spec['constants'].items():
      if constant_name.startswith('ACT_') and \
          not any(constant_name.startswith(s) for s in ['ACT_FLAG_', 'ACT_GROUP_', 'ACT_ID_']):
        action_name = constant_name.lower()[len('act_'):].replace('_', ' ')
        self.action_names[constant['value']] = action_name

    self.addr_to_symbol = {}
    for symbol in self.game.memory.data_spec['globals']:
      addr = self.game.memory.symbol(symbol)
      if not addr.is_null:
        self.addr_to_symbol[addr] = symbol

  def _set_edits(self, edits: Edits) -> None:
    if hasattr(self, 'timeline'):
      del self.timeline
    gc.collect() # Force garbage collection of game state slots

    self.edits = edits
    self.scripts = Scripts()
    self.controller = Controller.sequence(
      EditController(self.data_variables, self.edits),
      ScriptController(self.scripts),
    )

    self.timeline = Timeline(
      self.game,
      self.controller,
      slot_capacity = 20,
    )

    data_accessor = DataVariableAccessor(
      self.timeline,
      self.data_variables,
      self.addr_to_symbol,
      self.edits,
    )
    script_accessor = ScriptVariableAccessor(self.scripts)

    def choose_accessor(variable: Variable) -> VariableAccessor:
      if variable.name == 'wafel-script':
        return script_accessor
      else:
        return data_accessor

    range_edit_accessor = RangeEditAccessor(
      VariableAccessor.combine(choose_accessor),
      highlight_single = lambda variable: not variable.name.startswith('input-'),
    )
    self.accessor: VariableAccessor = range_edit_accessor
    self.drag_handler = range_edit_accessor

    for frame in range(len(self.edits)):
      for edit in list(self.edits.get_edits(frame)):
        self.accessor.set(edit.variable, edit.value)

    self._selected_frame = 0
    if config.dev_mode:
      self._selected_frame = 1580
    self.selected_frame_callbacks: List[Callable[[int], None]] = []

    self.play_speed = 0.0

    def set_hotspot(frame: int) -> None:
      self.timeline.set_hotspot('selected-frame', frame)
    self.on_selected_frame_change(set_hotspot)
    set_hotspot(self._selected_frame)

  # FrameSequence

  @property
  def selected_frame(self) -> int:
    return self._selected_frame

  @selected_frame.setter
  def selected_frame(self, frame: int) -> None:
    if frame != self._selected_frame:
      self._selected_frame = min(max(frame, 0), len(self.edits) - 1)
      for callback in list(self.selected_frame_callbacks):
        callback(self._selected_frame)

  def set_selected_frame(self, frame: int) -> None:
    self.selected_frame = frame

  def on_selected_frame_change(self, callback: Callable[[int], None]) -> None:
    self.selected_frame_callbacks.append(callback)

  @property
  def max_frame(self) -> int:
    return len(self.edits) - 1

  def extend_to_frame(self, frame: int) -> None:
    return self.edits.extend(frame + 1)

  def insert_frame(self, frame: int) -> None:
    self.edits.insert_frame(frame)
    if self.selected_frame >= frame:
      self.selected_frame += 1

  def delete_frame(self, frame: int) -> None:
    self.edits.delete_frame(frame)
    if self.selected_frame > frame or self.selected_frame >= len(self.edits):
      self.selected_frame -= 1

  def set_hotspot(self, name: str, frame: int) -> None:
    self.timeline.set_hotspot(name, frame)

  @overload
  def get(self, frame: int, path: Union[str, DataPath]) -> object:
    ...
  @overload
  def get(self, variable: Variable) -> object:
    ...
  def get(self, arg1, arg2=None):
    if isinstance(arg1, Variable):
      variable: Variable = arg1
      return self.accessor.get(variable)
    else:
      frame: int = arg1
      path: DataPath = arg2
      return self.timeline.get(frame, path)

  def get_object_type(self, frame: int, object_slot: int) -> Optional[ObjectType]:
    active = self.get(Variable('obj-active-flags-active', frame=frame, object=object_slot))
    if not active:
      return None
    behavior_addr = self.get(Variable('obj-behavior-ptr', frame=frame, object=object_slot))
    assert isinstance(behavior_addr, Address)
    return ObjectType(behavior_addr, self.addr_to_symbol[behavior_addr])

  def set(self, variable: Variable, value: object) -> None:
    self.accessor.set(variable, value)

  def reset(self, variable: Variable) -> None:
    self.accessor.reset(variable)

  def edited(self, variable: Variable) -> bool:
    return self.accessor.edited(variable)

  # VariableDisplayer

  def label(self, variable: Variable) -> str:
    if variable.name == 'wafel-script':
      return 'script'
    else:
      return assert_not_none(self.data_variables[variable].label)

  def column_header(self, variable: Variable) -> str:
    object_slot: Optional[int] = variable.args.get('object')
    surface_index: Optional[int] = variable.args.get('surface')

    if object_slot is not None:
      object_type: Optional[ObjectType] = variable.args.get('object_type')
      if object_type is None:
        return str(object_slot) + '\n' + self.label(variable)
      else:
        return str(object_slot) + ' - ' + object_type.name + '\n' + self.label(variable)

    elif surface_index is not None:
      return f'Surface {surface_index}\n{self.label(variable)}'

    else:
      return self.label(variable)
