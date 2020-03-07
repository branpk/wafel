from typing import *
import math
import contextlib

import imgui as ig

from wafel.model import Model
from wafel.graphics import CameraMode, Renderer, Camera, RotateCamera, \
  BirdsEyeCamera, RenderInfo, Viewport
from wafel.core import VariableParam


class MouseTracker:
  def __init__(self) -> None:
    self.dragging = False
    self.mouse_down = False
    self.mouse_pos = (0.0, 0.0)

  def is_mouse_in_window(self) -> bool:
    window_x, window_y = ig.get_window_position()
    window_w, window_h = ig.get_window_size()
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

    elif not mouse_was_down and self.mouse_down:
      window_x, window_y = ig.get_window_position()
      window_w, window_h = ig.get_window_size()
      if self.mouse_pos[0] >= window_x and self.mouse_pos[0] < window_x + window_w and \
          self.mouse_pos[1] >= window_y and self.mouse_pos[1] < window_y + window_h:
        self.dragging = True

    return (0, 0)

  def get_wheel_amount(self) -> float:
    if self.is_mouse_in_window():
      return ig.get_io().mouse_wheel
    else:
      return 0


class GameView:

  def __init__(self, model: Model, camera_mode: CameraMode) -> None:
    self.model = model
    self.camera_mode = camera_mode

    self.renderer = Renderer()
    self.mouse_tracker = MouseTracker()

    self.total_drag = [0.0, 0.0]
    self.zoom = 0


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
      target = mario_pos
      return BirdsEyeCamera(
        pos = [target[0], target[1] + 500, target[2]],
        span_y = 200 / math.pow(2, self.zoom),
      )

    else:
      raise NotImplementedError(self.camera_mode)


  def render(self, window_size: Tuple[int, int]) -> None:
    viewport_x, viewport_y = tuple(map(int, ig.get_window_position()))
    viewport_w, viewport_h = tuple(map(int, ig.get_window_size()))
    viewport_y = window_size[1] - viewport_y - viewport_h

    drag_amount = self.mouse_tracker.get_drag_amount()
    self.total_drag = (
      self.total_drag[0] + drag_amount[0],
      self.total_drag[1] + drag_amount[1],
    )
    self.zoom += self.mouse_tracker.get_wheel_amount() / 5

    with contextlib.ExitStack() as stack:
      root_state = stack.enter_context(self.model.timeline[self.model.selected_frame])
      neighbor_states = [
        stack.enter_context(self.model.timeline[self.model.selected_frame + i])
          for i in range(-5, 31)
            if self.model.selected_frame + i in range(len(self.model.timeline))
      ]

      self.renderer.render(RenderInfo(
        self.model.lib,
        Viewport(viewport_x, viewport_y, viewport_w, viewport_h),
        self.compute_camera(),
        root_state,
        neighbor_states,
      ))
