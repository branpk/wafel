import ctypes as C

from wafel.util import *


class GameLib:
  def __init__(self, spec: dict, dll: C.CDLL) -> None:
    self.spec = spec
    self.dll = dll

  def state_new(self) -> int:
    return dcast(int, self.dll.sm64_state_new())

  def state_raw_copy(self, dst: int, src: int) -> None:
    self.dll.sm64_state_raw_copy(dst, src)

  def state_update(self, addr: int) -> None:
    self.dll.sm64_state_update(addr)

  def concrete_type(self, type_: dict) -> dict:
    while type_['kind'] == 'symbol':
      type_ = self.spec['types'][type_['namespace']][type_['name']]
    return type_
