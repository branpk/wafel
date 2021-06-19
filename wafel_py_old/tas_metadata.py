from typing import *
from dataclasses import dataclass

@dataclass
class TasMetadata:
  game_version: str
  title: str
  authors: str
  description: str
  rerecords: Optional[int] = None
