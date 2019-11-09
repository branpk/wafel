from typing import *

from wafel.core import VariableId, Edits, Variables


class InputFile:
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
    self.edits: List[List[Tuple[VariableId, Any]]] = []

  def get_edits(self, frame: int) -> List[Tuple[VariableId, Any]]:
    while frame >= len(self.edits):
      self.edits.append([])
    return self.edits[frame]

  def to_edits(self, variables: Variables) -> Edits:
    edits = Edits()
    for frame in range(len(self.edits)):
      for variable_id, value in self.get_edits(frame):
        edits.edit(frame, variables[variable_id], value)
    return edits
