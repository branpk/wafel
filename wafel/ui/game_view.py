from typing import *
import math
import contextlib

import ext_modules.graphics as c_graphics

import wafel.imgui as ig
from wafel.model import Model
from wafel.core import VariableParam
from wafel.util import *
from wafel.local_state import use_state, use_state_with
from wafel.graphics import render_game


# TODO: Rename to game_view_overlay. Reduce parameters to minimum (don't require full Model)


class MouseTracker:
  def __init__(self) -> None:
    self.dragging = False
    self.mouse_down = False
    self.mouse_pos = (0.0, 0.0)

  def is_mouse_in_window(self) -> bool:
    window_x, window_y = cast(Iterable[float], ig.get_window_position())
    window_w, window_h = cast(Iterable[float], ig.get_window_size())
    return self.mouse_pos[0] >= window_x and self.mouse_pos[0] < window_x + window_w and \
      self.mouse_pos[1] >= window_y and self.mouse_pos[1] < window_y + window_h

  def get_drag_amount(self) -> Tuple[float, float]:
    mouse_was_down = self.mouse_down
    last_mouse_pos = self.mouse_pos
    self.mouse_down = ig.is_mouse_down()
    self.mouse_pos = ig.get_mouse_pos()

    if self.dragging:
      if not self.mouse_down:
        self.dragging = False
      return (
        self.mouse_pos[0] - last_mouse_pos[0],
        self.mouse_pos[1] - last_mouse_pos[1],
      )

    elif not mouse_was_down and self.mouse_down and not ig.is_any_item_hovered():
      window_x, window_y = ig.get_window_position()
      window_w, window_h = ig.get_window_size()
      if self.mouse_pos[0] >= window_x and self.mouse_pos[0] < window_x + window_w and \
          self.mouse_pos[1] >= window_y and self.mouse_pos[1] < window_y + window_h:
        self.dragging = True

    return (0, 0)

  def get_wheel_amount(self) -> float:
    if self.is_mouse_in_window():
      return cast(float, ig.get_io().mouse_wheel)
    else:
      return 0


def angle_to_direction(pitch: float, yaw: float) -> Tuple[float, float, float]:
  return (
    math.cos(pitch) * math.sin(yaw),
    math.sin(pitch),
    math.cos(pitch) * math.cos(yaw),
  )


def get_viewport(framebuffer_size: Tuple[int, int]) -> c_graphics.Viewport:
  window_pos = tuple(map(int, ig.get_window_position()))
  window_size = tuple(map(int, ig.get_window_size()))

  viewport = c_graphics.Viewport()
  viewport.pos.x = window_pos[0]
  viewport.pos.y = framebuffer_size[1] - window_pos[1] - window_size[1]
  viewport.size.x = window_size[0]
  viewport.size.y = window_size[1]

  return viewport


def get_mario_pos(model: Model) -> Tuple[float, float, float]:
  with model.timeline[model.selected_frame] as state:
    args = { VariableParam.STATE: state }
    return (
      model.variables['mario-pos-x'].get(args),
      model.variables['mario-pos-y'].get(args),
      model.variables['mario-pos-z'].get(args),
    )


def render_game_view_rotate(
  id: str,
  framebuffer_size: Tuple[int, int],
  model: Model,
) -> None:
  ig.push_id(id)

  mouse_state = use_state('mouse-state', MouseTracker()).value
  pitch = use_state('pitch', 0.0)
  yaw = use_state('yaw', 0.0)
  zoom = use_state('zoom', 0.0)

  drag_amount = mouse_state.get_drag_amount()
  pitch.value -= drag_amount[1] / 200
  yaw.value -= drag_amount[0] / 200
  zoom.value += mouse_state.get_wheel_amount() / 5

  target = get_mario_pos(model)
  offset = 1500 * math.pow(0.5, zoom.value)
  face_direction = angle_to_direction(pitch.value, yaw.value)
  camera_pos = (
    target[0] - offset * face_direction[0],
    target[1] - offset * face_direction[1],
    target[2] - offset * face_direction[2],
  )

  camera = c_graphics.RotateCamera()
  camera.pos = c_graphics.vec3(*camera_pos)
  camera.pitch = pitch.value
  camera.yaw = yaw.value
  camera.fov_y = math.radians(45)

  render_game(model, get_viewport(framebuffer_size), c_graphics.Camera(camera))

  ig.pop_id()


def render_pos_y_slider(
  id: str,
  pos_y: float,
  mario_pos_y: float,
) -> Tuple[Optional[float], bool]:
  ig.push_id(id)

  ig.text('max y = %.f' % pos_y)
  ig.set_cursor_pos((ig.get_window_width() - 30, ig.get_cursor_pos().y))

  slider_pos = ig.get_cursor_pos()
  slider_width = 20
  slider_height = ig.get_content_region_available().y
  slider_range = range(-8191, 8192)

  mario_icon_x = ig.get_cursor_pos().x
  t = (mario_pos_y - slider_range.start) / len(slider_range)
  mario_icon_y = ig.get_cursor_pos().y + (1 - t) * slider_height
  ig.set_cursor_pos((mario_icon_x, mario_icon_y))
  reset = ig.button('M', width=slider_width)

  ig.set_cursor_pos(slider_pos)
  changed, value = ig.v_slider_float(
    '##slider',
    width = slider_width,
    height = slider_height,
    value = pos_y,
    min_value = slider_range.start,
    max_value = slider_range.stop - 1,
    format = '',
  )
  new_y = value if changed else None

  ig.pop_id()
  return new_y, reset


def render_game_view_birds_eye(
  id: str,
  framebuffer_size: Tuple[int, int],
  model: Model,
) -> None:
  ig.push_id(id)

  # TODO: Should zoom in on mouse when uncentered
  mouse_state = use_state('mouse-state', MouseTracker()).value
  zoom = use_state('zoom', 0.0)
  target: Ref[Optional[Tuple[float, float]]] = use_state('target', None)
  pos_y: Ref[Optional[float]] = use_state('pos-y', None)

  drag_amount = mouse_state.get_drag_amount()
  zoom.value += mouse_state.get_wheel_amount() / 5
  world_span_x = 200 / math.pow(2, zoom.value)

  viewport = get_viewport(framebuffer_size)

  mario_pos = get_mario_pos(model)

  # Camera xz

  camera_xz = (mario_pos[0], mario_pos[2])
  if target.value is not None:
    camera_xz = target.value

  if drag_amount != (0.0, 0.0):
    world_span_z = world_span_x * viewport.size.x / viewport.size.y
    if target.value is None:
      target.value = (mario_pos[0], mario_pos[2])
    target.value = (
      camera_xz[0] + drag_amount[1] * world_span_x / viewport.size.y,
      camera_xz[1] - drag_amount[0] * world_span_z / viewport.size.x,
    )
    camera_xz = target.value

  if target.value is not None:
    if ig.button('Center'):
      target.value = None

  # Camera y

  camera_y = mario_pos[1] + 500 if pos_y.value is None else pos_y.value

  ig.set_cursor_pos((viewport.size.x - 100, 10))
  ig.begin_child('##y-slider')
  new_y, reset = render_pos_y_slider('y-slider', camera_y, mario_pos[1])
  if reset:
    pos_y.value = None
  elif new_y is not None:
    pos_y.value = new_y
    camera_y = pos_y.value
  ig.end_child()

  camera = c_graphics.BirdsEyeCamera()
  camera.pos = c_graphics.vec3(camera_xz[0], camera_y, camera_xz[1])
  camera.span_y = world_span_x

  render_game(model, viewport, c_graphics.Camera(camera))

  ig.pop_id()


__all__ = ['render_game_view_rotate', 'render_game_view_birds_eye']
