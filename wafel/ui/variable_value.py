from typing import *

import imgui as ig

from wafel.variable_format import VariableFormatter
from wafel.util import *


T = TypeVar('T')

def render_variable_value(
  id: str,
  value: T,
  formatter: VariableFormatter,
  size: Tuple[int, int],
  highlight: bool = False,
) -> Maybe[T]:
  ig.push_id(id)

  ig.pop_id()
  return Just(value)
