
class GameState:
  def __init__(self, spec: dict, frame: int, addr: int) -> None:
    self.spec = spec
    self.frame = frame
    self.addr = addr
