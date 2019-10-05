from enum import Enum, auto
from typing import *


class VariableParam(Enum):
  STATE = auto()
  OBJECT = auto()

VariableArgs = Dict[VariableParam, Any]
