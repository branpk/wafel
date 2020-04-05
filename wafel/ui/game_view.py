from typing import *
import math
import contextlib
import time

import glfw

import ext_modules.graphics as cg

import wafel.imgui as ig
from wafel.model import Model
from wafel.util import *
from wafel.local_state import use_state, use_state_with
from wafel.graphics import render_game
from wafel.core import DataPath, Address, AccessibleMemory
from wafel.variable import Variable
from wafel.bindings import input_float


# TODO: Rename to game_view_overlay. Reduce parameters to minimum (don't require full Model)


class MouseTracker:
  def __init__(self) -> None:
    self.dragging = False
    self.mouse_down = False
    self.mouse_pos = (0.0, 0.0)

  def is_mouse_in_window(self) -> bool:
    if not ig.global_mouse_capture():
      return False
    window_x, window_y = cast(Iterable[float], ig.get_window_position())
    window_w, window_h = cast(Iterable[float], ig.get_window_size())
    return self.mouse_pos[0] >= window_x and self.mouse_pos[0] < window_x + window_w and \
      self.mouse_pos[1] >= window_y and self.mouse_pos[1] < window_y + window_h

  def get_drag_amount(self) -> Tuple[float, float]:
    mouse_was_down = self.mouse_down
    last_mouse_pos = self.mouse_pos
    self.mouse_down = ig.is_mouse_down() and ig.global_mouse_capture()
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


Vec3f = Tuple[float, float, float]


def angle_to_direction(pitch: float, yaw: float) -> Vec3f:
  return (
    math.cos(pitch) * math.sin(yaw),
    math.sin(pitch),
    math.cos(pitch) * math.cos(yaw),
  )


def direction_to_angle(dir: Vec3f) -> Tuple[float, float]:
  xz = math.sqrt(dir[0] * dir[0] + dir[2] * dir[2])
  pitch = math.atan2(dir[1], xz)
  yaw = math.atan2(dir[0], dir[2])
  return pitch, yaw


def get_viewport(framebuffer_size: Tuple[int, int]) -> cg.Viewport:
  window_pos = tuple(map(int, ig.get_window_position()))
  window_size = tuple(map(int, ig.get_window_size()))

  viewport = cg.Viewport()
  viewport.pos.x = window_pos[0]
  viewport.pos.y = framebuffer_size[1] - window_pos[1] - window_size[1]
  viewport.size.x = window_size[0]
  viewport.size.y = window_size[1]

  return viewport


def get_mario_pos(model: Model) -> Vec3f:
  return (
    dcast(float, model.get(Variable('mario-pos-x').at(frame=model.selected_frame))),
    dcast(float, model.get(Variable('mario-pos-y').at(frame=model.selected_frame))),
    dcast(float, model.get(Variable('mario-pos-z').at(frame=model.selected_frame))),
  )


def move_toward(x: Vec3f, target: Vec3f, delta: float) -> Vec3f:
  remaining = (target[0] - x[0], target[1] - x[1], target[2] - x[2])
  distance = math.sqrt(sum(c ** 2 for c in remaining))
  if distance <= delta + 0.0001:
    return target
  return (
    x[0] + delta * remaining[0] / distance,
    x[1] + delta * remaining[1] / distance,
    x[2] + delta * remaining[2] / distance,
  )


def get_normalized_mouse_pos() -> Optional[Tuple[float, float]]:
  if not ig.global_mouse_capture():
    return None
  window_pos = tuple(map(int, ig.get_window_position()))
  window_size = tuple(map(int, ig.get_window_size()))
  mouse_pos = tuple(map(float, ig.get_mouse_pos()))
  mouse_pos = (
    mouse_pos[0] - window_pos[0],
    window_size[1] - mouse_pos[1] + window_pos[1],
  )
  mouse_pos = (
    2 * mouse_pos[0] / window_size[0] - 1,
    2 * mouse_pos[1] / window_size[1] - 1,
  )
  if any(c < -1 or c > 1 for c in mouse_pos):
    return None
  return mouse_pos


def get_mouse_ray(camera: cg.RotateCamera) -> Optional[Tuple[Vec3f, Vec3f]]:
  window_size = tuple(map(int, ig.get_window_size()))
  mouse_pos = get_normalized_mouse_pos()
  if mouse_pos is None:
    return None

  forward_dir = angle_to_direction(camera.pitch, camera.yaw)
  up_dir = angle_to_direction(camera.pitch + math.pi / 2, camera.yaw)
  right_dir = angle_to_direction(0, camera.yaw - math.pi / 2)

  top = math.tan(camera.fov_y / 2)
  right = top * window_size[0] / window_size[1]

  mouse_dir = tuple(
    forward_dir[i] +
      top * mouse_pos[1] * up_dir[i] +
      right * mouse_pos[0] * right_dir[i]
        for i in range(3)
  )
  mag = math.sqrt(sum(c ** 2 for c in mouse_dir))
  mouse_dir = (mouse_dir[0] / mag, mouse_dir[1] / mag, mouse_dir[2] / mag)

  return ((camera.pos.x, camera.pos.y, camera.pos.z), mouse_dir)


def get_mouse_world_pos_birds_eye(camera: cg.BirdsEyeCamera) -> Optional[Tuple[float, float]]:
  window_size = tuple(map(int, ig.get_window_size()))
  mouse_pos = get_normalized_mouse_pos()
  if mouse_pos is None:
    return None

  world_span_x = camera.span_y
  world_span_z = camera.span_y * window_size[0] / window_size[1]
  return (
    camera.pos.x + mouse_pos[1] * world_span_x / 2,
    camera.pos.z + mouse_pos[0] * world_span_z / 2,
  )


def trace_ray(model: Model, ray: Tuple[Vec3f, Vec3f]) -> Optional[int]:
  def get_field_offset(path: str) -> int:
    return model.game.path(path).edges[-1].value

  memory = model.game.memory
  assert isinstance(memory, AccessibleMemory)

  with model.timeline.request_base(model.selected_frame) as state:
    surface_pool_addr = dcast(Address, state.get('sSurfacePool'))
    if surface_pool_addr.is_null:
      return None

    index = cg.trace_ray_to_surface(
      cg.vec3(*ray[0]),
      cg.vec3(*ray[1]),
      memory.address_to_location(state.slot, surface_pool_addr),
      memory.data_spec['types']['struct']['Surface']['size'],
      state.get('gSurfacesAllocated'),
      get_field_offset,
    )

  return None if index < 0 else index


def use_rotational_camera(
  framebuffer_size: Tuple[int, int],
  model: Model,
) -> cg.RotateCamera:
  mouse_state = use_state('mouse-state', MouseTracker()).value
  target: Ref[Optional[Vec3f]] = use_state('target', None)
  target_vel: Ref[Optional[Vec3f]] = use_state('target-vel', None)
  pitch = use_state('pitch', 0.0)
  yaw = use_state('yaw', 0.0)
  zoom = use_state('zoom', 0.0)
  prev_frame_time = use_state_with('prev-frame-time', time.time)
  lock_to_in_game = use_state('lock-to-in-game', False)

  delta_time = time.time() - prev_frame_time.value
  prev_frame_time.value = time.time()

  drag_amount = mouse_state.get_drag_amount()
  pitch.value -= drag_amount[1] / 200
  yaw.value -= drag_amount[0] / 200
  wheel_amount = mouse_state.get_wheel_amount()
  zoom.value += wheel_amount / 5
  zoom.value = min(zoom.value, 7.0)

  mario_pos = get_mario_pos(model)
  target_pos = mario_pos if target.value is None else target.value

  fov_y = math.radians(45)

  if drag_amount != (0.0, 0.0) or wheel_amount != 0.0:
    lock_to_in_game.value = False

  if lock_to_in_game.value:
    target_pos = cast(Vec3f, model.get(model.selected_frame, 'gLakituState.focus'))
    target.value = target_pos
    camera_pos = cast(Vec3f, model.get(model.selected_frame, 'gLakituState.pos'))
    dpos = (
      target_pos[0] - camera_pos[0],
      target_pos[1] - camera_pos[1],
      target_pos[2] - camera_pos[2],
    )
    pitch.value, yaw.value = direction_to_angle(dpos)
    offset = math.sqrt(sum(c ** 2 for c in dpos))
    if offset > 0.001:
      zoom.value = math.log(offset / 1500, 0.5)
    fov_y = math.radians(cast(float, model.get(model.selected_frame, 'sFOVState.fov')))

  offset = 1500 * math.pow(0.5, zoom.value)
  face_direction = angle_to_direction(pitch.value, yaw.value)

  move = [0.0, 0.0, 0.0] # forward, up, right
  move[0] += input_float('3d-camera-move-f')
  move[0] -= input_float('3d-camera-move-b')
  move[1] += input_float('3d-camera-move-u')
  move[1] -= input_float('3d-camera-move-d')
  move[2] += input_float('3d-camera-move-r')
  move[2] -= input_float('3d-camera-move-l')

  if move != [0.0, 0.0, 0.0] or (target.value is not None and not lock_to_in_game.value):
    mag = math.sqrt(sum(c ** 2 for c in move))
    if mag > 1:
      move = [c / mag for c in move]

    max_speed = 50.0 * delta_time * math.sqrt(offset)
    f = (math.sin(yaw.value), 0, math.cos(yaw.value))
    u = (0, 1, 0)
    r = (-f[2], 0, f[0])
    end_vel = cast(Vec3f, tuple(
        max_speed * move[0] * f[i] + max_speed * move[1] * u[i] + max_speed * move[2] * r[i]
          for i in range(3)
      ))

    accel = 10.0 * delta_time * math.sqrt(offset)
    current_vel = target_vel.value or (0.0, 0.0, 0.0)
    target_vel.value = move_toward(current_vel, end_vel, accel)
    target.value = (
      target_pos[0] + target_vel.value[0],
      target_pos[1] + target_vel.value[1],
      target_pos[2] + target_vel.value[2],
    )
    target_pos = target.value
    lock_to_in_game.value = False

  if ig.disableable_button('Lock to Mario', enabled=target.value is not None):
    target.value = None
    target_vel.value = None
    lock_to_in_game.value = False
  ig.same_line()
  if ig.disableable_button('Lakitu', enabled=not lock_to_in_game.value):
    lock_to_in_game.value = True

  camera_pos = (
    target_pos[0] - offset * face_direction[0],
    target_pos[1] - offset * face_direction[1],
    target_pos[2] - offset * face_direction[2],
  )

  camera = cg.RotateCamera()
  camera.pos = cg.vec3(*camera_pos)
  camera.target = cg.vec3(*target_pos)
  camera.pitch = pitch.value
  camera.yaw = yaw.value
  camera.fov_y = fov_y
  if target.value is not None and not lock_to_in_game.value:
    camera.render_target = True # TODO: Should be a scene config

  return camera


def render_game_view_rotate(
  id: str,
  framebuffer_size: Tuple[int, int],
  model: Model,
  wall_hitbox_radius: float,
  hovered_surface: Optional[int],
  hidden_surfaces: Set[int],
) -> Optional[int]:
  ig.push_id(id)

  log.timer.begin('overlay')
  camera = use_rotational_camera(framebuffer_size, model)
  model.rotational_camera_yaw = int(camera.yaw * 0x8000 / math.pi)
  log.timer.end()

  mouse_ray = get_mouse_ray(camera)
  if mouse_ray is None:
    new_hovered_surface = None
  else:
    new_hovered_surface = trace_ray(model, mouse_ray)

  render_game(
    model,
    get_viewport(framebuffer_size),
    cg.Camera(camera),
    wall_hitbox_radius,
    hovered_surface=hovered_surface,
    hidden_surfaces=hidden_surfaces,
  )

  ig.pop_id()
  return new_hovered_surface


def render_game_view_in_game(
  id: str,
  framebuffer_size: Tuple[int, int],
  model: Model,
) -> None:
  ig.push_id(id)

  camera = use_rotational_camera(framebuffer_size, model)

  # Invalidate frame to ensure no rendering state gets copied to other slots
  prev_frame = max(model.selected_frame - 1, 0)
  with model.timeline.request_base(prev_frame, invalidate=True) as state:
    # TODO: Override fov (so that it stays at 45 when not in in-game mode)
    model.game.path('gOverrideCamera.enabled').set(state.slot, True)
    model.game.path('gOverrideCamera.pos[0]').set(state.slot, camera.pos.x)
    model.game.path('gOverrideCamera.pos[1]').set(state.slot, camera.pos.y)
    model.game.path('gOverrideCamera.pos[2]').set(state.slot, camera.pos.z)
    model.game.path('gOverrideCamera.focus[0]').set(state.slot, camera.target.x)
    model.game.path('gOverrideCamera.focus[1]').set(state.slot, camera.target.y)
    model.game.path('gOverrideCamera.focus[2]').set(state.slot, camera.target.z)

    sm64_update_and_render = model.game.memory.symbol('sm64_update_and_render').absolute
    cg.update_and_render(get_viewport(framebuffer_size), sm64_update_and_render)

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
  wall_hitbox_radius: float,
  hovered_surface: Optional[int],
  hidden_surfaces: Set[int],
) -> Optional[int]:
  ig.push_id(id)

  # TODO: Should zoom in on mouse when uncentered
  mouse_state = use_state('mouse-state', MouseTracker()).value
  zoom = use_state('zoom', -4.5)
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

  if ig.disableable_button('Lock to Mario', enabled=target.value is not None):
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

  camera = cg.BirdsEyeCamera()
  camera.pos = cg.vec3(camera_xz[0], camera_y, camera_xz[1])
  camera.span_y = world_span_x

  # Mouse xz
  mouse_world_pos = get_mouse_world_pos_birds_eye(camera)
  mouse_ray: Optional[Tuple[Vec3f, Vec3f]]
  if mouse_world_pos is not None:
    ig.set_cursor_pos((10, viewport.size.y - 25))
    ig.text('(x, z) = (%.3f, %.3f)' % mouse_world_pos)
    mouse_ray = ((mouse_world_pos[0], camera.pos.y, mouse_world_pos[1]), (0, -1, 0))
  else:
    mouse_ray = None

  if mouse_ray is None:
    new_hovered_surface = None
  else:
    new_hovered_surface = trace_ray(model, mouse_ray)

  render_game(
    model,
    viewport,
    cg.Camera(camera),
    wall_hitbox_radius,
    hovered_surface=hovered_surface,
    hidden_surfaces=hidden_surfaces,
  )

  ig.pop_id()
  return new_hovered_surface


__all__ = ['render_game_view_rotate', 'render_game_view_in_game', 'render_game_view_birds_eye']
