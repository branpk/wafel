from typing import *

from wafel.game_lib import GameLib


class GameState:
  def __init__(self, lib: GameLib, base_addr: int, frame: int, addr: int) -> None:
    self.lib = lib
    self.base_addr = base_addr
    self.frame = frame
    self.addr = addr


ObjectId = int

class Object:
  def __init__(self, addr: int) -> None:
    self.addr = addr
