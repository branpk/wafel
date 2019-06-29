import ctypes
from typing import List, IO, Optional

from butter.game_state import GameState


class SequenceItem:
  def apply(self, state: GameState) -> None:
    raise NotImplementedError()


class Input(SequenceItem):
  def __init__(self, stick_x: int, stick_y: int, buttons: int):
    self.stick_x = stick_x
    self.stick_y = stick_y
    self.buttons = buttons

  def apply(self, state: GameState) -> None:
    # TODO: Better system for this
    globals = state.spec['types']['struct']['SM64State']['fields']

    controller = state.addr + globals['gControllerPads']['offset']
    os_cont_pad = state.spec['types']['typedef']['OSContPad']['fields']
    controller_button = ctypes.cast(controller + os_cont_pad['button']['offset'], ctypes.POINTER(ctypes.c_uint16))
    controller_stick_x = ctypes.cast(controller + os_cont_pad['stick_x']['offset'], ctypes.POINTER(ctypes.c_int8))
    controller_stick_y = ctypes.cast(controller + os_cont_pad['stick_y']['offset'], ctypes.POINTER(ctypes.c_int8))

    controller_button[0] = self.buttons
    controller_stick_x[0] = self.stick_x
    controller_stick_y[0] = self.stick_y


def read_byte(f: IO[bytes]) -> Optional[int]:
  return (f.read(1) or [None])[0]


def read_big_short(f: IO[bytes]) -> Optional[int]:
  byte1 = read_byte(f)
  byte2 = read_byte(f)
  if byte1 is None or byte2 is None:
    return None
  return byte1 << 8 | byte2


class InputSequence:
  @staticmethod
  def from_m64(m64: IO[bytes]) -> 'InputSequence':
    inputs = InputSequence()

    m64.seek(0x400)
    i = 0
    while True:
      buttons = read_big_short(m64)
      stick_x = read_byte(m64)
      stick_y = read_byte(m64)
      if buttons is None or stick_x is None or stick_y is None:
        break
      else:
        inputs[i].append(Input(stick_x, stick_y, buttons))
      i += 1

    return inputs

  def __init__(self):
    self._items: List[List[SequenceItem]] = []

  def __len__(self) -> int:
    return len(self._items)

  def __getitem__(self, frame: int) -> List[SequenceItem]:
    while frame >= len(self._items):
      self._items.append([])
    return self._items[frame]

  def apply(self, state: GameState) -> None:
    for item in self[state.frame]:
      item.apply(state)
