from typing import *


class GameState:
  def __init__(self, spec: dict, base_addr: int, frame: int, addr: int) -> None:
    self.spec = spec
    self.base_addr = base_addr
    self.frame = frame
    self.addr = addr


ObjectId = int

class Object:
  def __init__(self, addr: int) -> None:
    self.addr = addr

class ObjectType:
  def __init__(
    self,
    name: str,
    behavior: List[str],
  ) -> None:
    self.name = name
    self.behavior = behavior
