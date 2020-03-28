from typing import *
import functools

import imgui as ig

from wafel.util import *
import wafel.config as config

_stack: List[Tuple[str, Any]] = []
_frames_without_modal: int = 0

def _unconditional_begin_call(name: str) -> Any:
  ig_func = getattr(ig, name)
  def func(*args, **kwargs):
    _stack.append((name, (args, kwargs)))
    return ig_func(*args, **kwargs)
  return func

def _begin_child(*args, **kwargs):
  _stack.append(('begin_child', (args, kwargs)))
  fixed_args = list(args)
  fixed_args[0] = str(ig.get_id(fixed_args[0]))
  return ig.begin_child(*fixed_args, **kwargs)

def _conditional_begin_call(name: str) -> Any:
  ig_func = getattr(ig, name)
  def func(*args, **kwargs):
    result = ig_func(*args, **kwargs)
    if result:
      _stack.append((name, (args, kwargs)))
    return result
  return func

def _begin_popup_modal(*args, **kwargs):
  global _frames_without_modal
  result = ig.begin_popup_modal(*args, **kwargs)
  if result[0]:
    _stack.append(('begin_popup_modal', (args, kwargs)))
    _frames_without_modal = 0
  return result

def _end_call(name: str) -> Any:
  if name.startswith('end'):
    matching = 'begin' + name[len('end'):]
  elif name.startswith('pop'):
    matching = 'push' + name[len('pop'):]
  else:
    assert False, name

  if name == 'end_popup_context_item':
    name = 'end_popup'
  elif name == 'end_popup_modal':
    name = 'end_popup'
  ig_func = getattr(ig, name)

  def func():
    if len(_stack) == 0 or _stack[-1][0] != matching:
      for item in _stack:
        log.error(' ', item[0], *item[1])
      assert False, 'Expected ' + matching
    _stack.pop()
    ig_func()

  return func

def _end_frame() -> None:
  global _frames_without_modal
  assert len(_stack) == 0, _stack
  _frames_without_modal += 1

@functools.lru_cache(maxsize=None)
def get_func(name: str) -> Any:
  if name == 'begin_child':
    return _begin_child
  elif name == 'begin_popup_modal':
    return _begin_popup_modal
  elif name == 'begin' or name.startswith('push'):
    return _unconditional_begin_call(name)
  elif name.startswith('begin'):
    return _conditional_begin_call(name)
  elif name == 'end_frame':
    return _end_frame
  elif name.startswith('end') or name.startswith('pop'):
    return _end_call(name)
  else:
    return getattr(ig, name)

def __getattr__(name: str) -> Any:
  return get_func(name)

# This tricks mypy into seeing the __getattr__ definition while avoiding
# unnecessary indirection
__getattr__ = get_func

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

def global_keyboard_capture() -> bool:
  # TOOD: Mouse thing is weird behavior but probably fine?
  if _frames_without_modal < 2:
    return False
  return not ig.get_io().want_capture_keyboard or ig.is_mouse_down()

def global_mouse_capture() -> bool:
  return _frames_without_modal >= 2

def global_input_capture() -> bool:
  return global_keyboard_capture()

_clipboard_length: int

def get_clipboard_length() -> int:
  return _clipboard_length

def set_clipboard_length(length: int) -> None:
  global _clipboard_length
  _clipboard_length = length
