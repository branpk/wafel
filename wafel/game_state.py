
class GameState:
  def __init__(self, spec: dict, base_addr: int, frame: int, addr: int) -> None:
    self.spec = spec
    self.base_addr = base_addr
    self.frame = frame
    self.addr = addr

class Object:
  def __init__(self, addr: int) -> None:
    self.addr = addr
