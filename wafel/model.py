from typing import *
from ctypes import cdll
import json
import gc
import os
import weakref

import ext_modules.util as c_util

import wafel.config as config
from wafel.core import Game, Timeline, load_dll_game, Controller, Address, DataPath, \
  AccessibleMemory, Slot
from wafel.variable import Variable
from wafel.edit import Edits
from wafel.object_type import ObjectType
from wafel.loading import Loading
from wafel.util import *
from wafel.script import Scripts, ScriptController
from wafel.data_variables import DataVariables


class EditController(Controller):
  def __init__(self, data_variables: DataVariables, edits: Edits) -> None:
    super().__init__()
    self.data_variables = data_variables
    self.edits = edits
    self.edits.on_edit(self.weak_notify)

  def apply(self, game: Game, frame: int, slot: Slot) -> None:
    for edit in self.edits.get_edits(frame):
      self.data_variables.set_raw(slot, edit.variable, edit.value)


class Model:

  def __init__(self) -> None:
    self.game_version: str
    self.timeline: Timeline
    self.rotational_camera_yaw = 0

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

    self._selected_frame = 0
    if config.dev_mode:
      self._selected_frame = 1580
    self.selected_frame_callbacks: List[Callable[[int], None]] = []

    self.play_speed = 0.0

    def set_hotspot(frame: int) -> None:
      self.timeline.set_hotspot('selected-frame', frame)
    self.on_selected_frame_change(set_hotspot)
    set_hotspot(self._selected_frame)

  @property
  def selected_frame(self) -> int:
    return self._selected_frame

  @selected_frame.setter
  def selected_frame(self, frame: int) -> None:
    self._selected_frame = min(max(frame, 0), len(self.edits) - 1)
    for callback in list(self.selected_frame_callbacks):
      callback(self._selected_frame)

  def on_selected_frame_change(self, callback: Callable[[int], None]) -> None:
    self.selected_frame_callbacks.append(callback)

  def insert_frame(self, index: int) -> None:
    self.edits.insert_frame(index)
    if self.selected_frame >= index:
      self.selected_frame += 1

  def delete_frame(self, index: int) -> None:
    self.edits.delete_frame(index)
    if self.selected_frame > index or self.selected_frame >= len(self.edits):
      self.selected_frame -= 1

  @overload
  def get(self, data: Union[str, DataPath, Variable]) -> object:
    ...
  @overload
  def get(self, frame: int, data: Union[str, DataPath, Variable]) -> object:
    ...
  def get(self, frame, data = None):
    if data is None:
      data = frame
      frame = self.selected_frame
    if isinstance(data, Variable):
      if data.name == 'wafel-script':
        return self.scripts.get(frame).source
      else:
        return self.data_variables.get(self.timeline, data.at(frame=frame))
    else:
      return self.timeline.get(frame, data)

  def get_object_type(self, frame: int, object_slot: int) -> Optional[ObjectType]:
    active = self.get(frame, Variable('obj-active-flags-active', object=object_slot))
    if not active:
      return None

    behavior_addr = self.get(frame, Variable('obj-behavior-ptr', object=object_slot))
    assert isinstance(behavior_addr, Address)
    return ObjectType(behavior_addr, self.addr_to_symbol[behavior_addr])

  def edit(self, frame: int, variable: Variable, data: object) -> None:
    if variable.name == 'wafel-script':
      self.scripts.set_frame_source(frame, dcast(str, data))
    else:
      self.edits.edit(frame, variable, data)

  def is_edited(self, frame: int, variable: Variable) -> bool:
    if variable.name == 'wafel-script':
      return self.scripts.is_edited(frame)
    else:
      return self.edits.is_edited(frame, variable)

  def reset(self, frame: int, variable: Variable) -> None:
    if variable.name == 'wafel-script':
      return self.scripts.reset_frame(frame)
    else:
      return self.edits.reset(frame, variable)

  def label(self, variable: Variable) -> str:
    if variable.name == 'wafel-script':
      return 'script'
    else:
      return assert_not_none(self.data_variables[variable].label)
