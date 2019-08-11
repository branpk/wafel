from butter.game_state import GameState

class RenderInfo:
  def __init__(
    self,
    current_state: GameState,
  ) -> None:
    self.current_state = current_state

  # TODO: Make expression evaluation available to ext module to compute offsets
