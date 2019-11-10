from typing import *

class TasMetadata:
  def __init__(
    self,
    game_version: str,
    title: str,
    authors: str,
    description: str,
    rerecords: Optional[int] = None,
  ) -> None:
    self.game_version = game_version
    self.title = title
    self.authors = authors
    self.description = description
    self.rerecords = rerecords
