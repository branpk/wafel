from typing import *
import math
import contextlib

import wafel.imgui as ig
from wafel.model import Model
from wafel.graphics import CameraMode, Renderer, Camera, RotateCamera, \
  BirdsEyeCamera, RenderInfo, Viewport
from wafel.core import VariableParam
from wafel.util import *
from wafel.local_state import use_state


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


class GameView:

  def __init__(self, model: Model, camera_mode: CameraMode) -> None:
    self.model = model
    self.camera_mode = camera_mode

    self.renderer = Renderer.get()
    self.mouse_tracker = MouseTracker()

    self.total_drag = (0.0, 0.0)
    self.zoom = 0.0

    self.birds_eye_target: Optional[Tuple[float, float]] = None
    self.birds_eye_y: Optional[float] = None


  def compute_camera(self) -> Camera:
    with self.model.timeline[self.model.selected_frame] as state:
      args = { VariableParam.STATE: state }
      mario_pos = [
        self.model.variables['mario-pos-x'].get(args),
        self.model.variables['mario-pos-y'].get(args),
        self.model.variables['mario-pos-z'].get(args),
      ]

    if self.camera_mode == CameraMode.ROTATE:
      target = mario_pos
      offset_dist = 1500 * math.pow(0.5, self.zoom)
      camera = RotateCamera(
        pos = [0.0, 0.0, 0.0],
        pitch = -self.total_drag[1] / 200,
        yaw = -self.total_drag[0] / 200,
        fov_y = math.radians(45)
      )
      face_dir = camera.face_dir()
      camera.pos = [target[i] - offset_dist * face_dir[i] for i in range(3)]
      return camera

    elif self.camera_mode == CameraMode.BIRDS_EYE:
      if self.birds_eye_target is None:
        pos_x, pos_z = mario_pos[0], mario_pos[2]
      else:
        pos_x, pos_z = self.birds_eye_target[0], self.birds_eye_target[1]
      if self.birds_eye_y is None:
        pos_y = mario_pos[1] + 500
      else:
        pos_y = self.birds_eye_y
      return BirdsEyeCamera(
        pos = [pos_x, pos_y, pos_z],
        span_y = 200 / math.pow(2, self.zoom),
      )

    else:
      raise NotImplementedError(self.camera_mode)


  def render(self, window_size: Tuple[int, int]) -> None:
    window_pos = tuple(map(int, ig.get_window_position()))
    viewport_w, viewport_h = tuple(map(int, ig.get_window_size()))
    viewport_x = window_pos[0]
    viewport_y = window_size[1] - window_pos[1] - viewport_h

    drag_amount = self.mouse_tracker.get_drag_amount()
    self.total_drag = (
      self.total_drag[0] + drag_amount[0],
      self.total_drag[1] + drag_amount[1],
    )
    self.zoom += self.mouse_tracker.get_wheel_amount() / 5
    # TODO: In bird's eye, should zoom in on mouse when uncentered

    camera = self.compute_camera()

    with self.model.timeline[self.model.selected_frame] as state:
      args = { VariableParam.STATE: state }
      mario_pos = [
        self.model.variables['mario-pos-x'].get(args),
        self.model.variables['mario-pos-y'].get(args),
        self.model.variables['mario-pos-z'].get(args),
      ]

    if self.camera_mode == CameraMode.BIRDS_EYE:
      assert isinstance(camera, BirdsEyeCamera)
      if drag_amount != (0.0, 0.0):
        span_y = camera.span_y
        span_x = span_y * viewport_w / viewport_h
        self.birds_eye_target = (
          camera.pos[0] + drag_amount[1] * span_y / viewport_h,
          camera.pos[2] - drag_amount[0] * span_x / viewport_w,
        )
      if self.birds_eye_target is not None:
        if ig.button('Center'):
          self.birds_eye_target = None

      ig.set_cursor_pos((viewport_w - 100, 10))
      ig.begin_child('##y-info')
      ig.text('max y = %.f' % camera.pos[1])
      ig.set_cursor_pos((ig.get_window_width() - 30, ig.get_cursor_pos().y))

      slider_pos = ig.get_cursor_pos()
      slider_width = 20
      slider_height = ig.get_content_region_available().y
      slider_range = range(-8191, 8192)

      mario_x = ig.get_cursor_pos().x
      t = (mario_pos[1] - slider_range.start) / len(slider_range)
      mario_y = ig.get_cursor_pos().y + (1 - t) * slider_height
      ig.set_cursor_pos((mario_x, mario_y))
      if ig.button('M', width=slider_width):
        self.birds_eye_y = None

      ig.set_cursor_pos(slider_pos)
      changed, new_y = ig.v_slider_float(
        '##y-slider',
        width = slider_width,
        height = slider_height,
        value = camera.pos[1],
        min_value = slider_range.start,
        max_value = slider_range.stop - 1,
        format = '',
      )
      if changed:
        self.birds_eye_y = new_y

      ig.end_child()

    # TODO: Extract out needed info for slots one-by-one instead of using nested
    # lookups

    with contextlib.ExitStack() as stack:
      root_state = stack.enter_context(self.model.timeline.get(self.model.selected_frame, allow_nesting=True))
      neighbor_states = [
        stack.enter_context(self.model.timeline.get(self.model.selected_frame + i, allow_nesting=True))
          for i in range(-5, 31)
            if self.model.selected_frame + i in range(len(self.model.timeline))
      ]

      self.renderer.render(RenderInfo(
        self.model.lib,
        Viewport(viewport_x, viewport_y, viewport_w, viewport_h),
        camera,
        root_state,
        neighbor_states,
      ))
