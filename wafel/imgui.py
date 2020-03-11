from typing import *

import imgui as ig

from wafel.util import *

_stack: List[Tuple[str, Any]] = []

# TODO: Test exceptions in begin_menu_bar, push_item_width, begin_drag_drop_source, begin_popup_context_item

def _unconditional_begin_call(name: str) -> Any:
  def func(*args, **kwargs):
    _stack.append((name, (args, kwargs)))
    return getattr(ig, name)(*args, **kwargs)
  return func

def _conditional_begin_call(name: str) -> Any:
  def func(*args, **kwargs):
    result = getattr(ig, name)(*args, **kwargs)
    if result:
      _stack.append((name, (args, kwargs)))
    return result
  return func

def _check_end_call(name: str) -> None:
  if name.startswith('end'):
    matching = 'begin' + name[len('end'):]
  elif name.startswith('pop'):
    matching = 'push' + name[len('pop'):]
  else:
    return
  if len(_stack) == 0 or _stack[-1][0] != matching:
    for item in _stack:
      log.error(' ', item[0], *item[1])
    assert False, 'Expected ' + matching
  _stack.pop()

def __getattr__(name: str) -> Any:
  if name in ['begin', 'begin_child'] or name.startswith('push'):
    return _unconditional_begin_call(name)
  elif name.startswith('begin'):
    return _conditional_begin_call(name)
  elif name == 'end_frame':
    assert len(_stack) == 0, _stack
  else:
    _check_end_call(name)

  if name == 'end_popup_context_item':
    name = 'end_popup'
  return getattr(ig, name)

def try_render(render: Callable[[], None]) -> None:
  stack_size = len(_stack)
  try:
    render()
  except:
    while len(_stack) > stack_size:
      begin_call, _ = _stack[-1]
      if begin_call.startswith('begin'):
        end_call = 'end' + begin_call[len('begin'):]
      elif begin_call.startswith('push'):
        end_call = 'pop' + begin_call[len('push'):]
      else:
        assert False, begin_call
      __getattr__(end_call)()
    raise
