from typing import *

from wafel.core.game_lib import GameLib
from wafel.core.cell_manager import Buffer, OwnedBuffer


class GameState:
  def __init__(
    self,
    lib: GameLib,
    frame: int,
    buffer: OwnedBuffer,
    base_buffer: Buffer,
  ) -> None:
    self.lib = lib
    self.frame = frame
    self.buffer_owned = buffer
    self.base_buffer = base_buffer

  @property
  def buffer(self) -> Buffer:
    return self.buffer_owned.value


ObjectId = int

class Object:
  def __init__(self, addr: int) -> None:
    self.addr = addr
