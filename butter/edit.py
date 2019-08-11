import ctypes
from typing import IO, Any, Optional, List

from butter.game_state import GameState
from butter.variable import Variable, VariableParam
from butter.reactive import Reactive, ReactiveValue


# TODO: Make InputSequence reactive


class Edit:
  def apply(self, state: GameState) -> None:
    raise NotImplementedError()


# TODO: Treat Inputs as variable edits instead
class Input(Edit):
  def __init__(self, stick_x: int, stick_y: int, buttons: int):
    self.stick_x = stick_x
    self.stick_y = stick_y
    self.buttons = buttons

  def apply(self, state: GameState) -> None:
    # TODO: Better system for this (make input variables for STATE instead of INPUT?)
    globals = state.spec['types']['struct']['SM64State']['fields']

    controller = state.addr + globals['gControllerPads']['offset']
    os_cont_pad = state.spec['types']['typedef']['OSContPad']['fields']
    controller_button = ctypes.cast(controller + os_cont_pad['button']['offset'], ctypes.POINTER(ctypes.c_uint16))
    controller_stick_x = ctypes.cast(controller + os_cont_pad['stick_x']['offset'], ctypes.POINTER(ctypes.c_int8))
    controller_stick_y = ctypes.cast(controller + os_cont_pad['stick_y']['offset'], ctypes.POINTER(ctypes.c_int8))

    controller_button[0] = self.buttons
    controller_stick_x[0] = self.stick_x
    controller_stick_y[0] = self.stick_y


class VariableEdit(Edit):
  def __init__(self, variable: Variable, value: Any) -> None:
    # TODO: Extra args (e.g. object id)
    assert variable.params == [VariableParam.STATE]
    self.variable = variable
    self.value = value

  def apply(self, state: GameState) -> None:
    self.variable.set(self.value, state)


def read_byte(f: IO[bytes]) -> Optional[int]:
  return (f.read(1) or [None])[0]


def read_big_short(f: IO[bytes]) -> Optional[int]:
  byte1 = read_byte(f)
  byte2 = read_byte(f)
  if byte1 is None or byte2 is None:
    return None
  return byte1 << 8 | byte2


class Edits:
  @staticmethod
  def from_m64(m64: IO[bytes]) -> 'Edits':
    edits = Edits()

    m64.seek(0x400)
    frame = 0
    while True:
      buttons = read_big_short(m64)
      stick_x = read_byte(m64)
      stick_y = read_byte(m64)
      if buttons is None or stick_x is None or stick_y is None:
        break
      else:
        edits.set_input(frame, Input(stick_x, stick_y, buttons))
      frame += 1

    return edits

  def __init__(self):
    self._items: List[List[Edit]] = []
    self.latest_edited_frame = ReactiveValue(-1)

  # TODO: Handle inputs better
  def get_input(self, frame: int) -> Input:
    return [item for item in self._get_edits(frame) if isinstance(item, Input)][-1]

  def set_input(self, frame: int, input: Input) -> None:
    self.add(frame, input)

  def add(self, frame: int, edit: Edit) -> None:
    self._get_edits(frame).append(edit)
    self.latest_edited_frame.change_value(frame)

  def _get_edits(self, frame: int) -> List[Edit]:
    while frame >= len(self._items):
      self._items.append([])
    return self._items[frame]

  def apply(self, state: GameState) -> None:
    for item in self._get_edits(state.frame):
      item.apply(state)
