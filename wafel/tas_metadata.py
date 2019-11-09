from typing import *

class TasMetadata:
  def __init__(
    self,
    game_version: str,
    title: str,
    authors: List[str],
    description: str,
  ) -> None:
    self.game_version = game_version
    self.title = title
    self.authors = authors
    self.description = description
