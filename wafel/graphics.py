from typing import *

import ext_modules.graphics as cg

from wafel.model import Model
import wafel.config as config
from wafel.core import DataPath, Address, AccessibleMemory
from wafel.util import *


_renderer: Optional[cg.Renderer] = None

def get_renderer() -> cg.Renderer:
  global _renderer
  if _renderer is None:
    cg.init_opengl()
    _renderer = cg.Renderer(config.assets_directory)
  return _renderer

def render_scene(scene: cg.Scene) -> None:
  get_renderer().render(scene)


def build_mario_path(model: Model, path_frames: range) -> cg.ObjectPath:
  mario_path = cg.ObjectPath()

  log.timer.begin('nodes')
  pos_x = model.game.path('gMarioState[].pos[0]')
  pos_y = model.game.path('gMarioState[].pos[1]')
  pos_z = model.game.path('gMarioState[].pos[2]')
  cg.object_path_add_nodes(
    mario_path,
    path_frames.start,
    path_frames.stop,
    lambda frame: model.timeline.get(frame, pos_x),
    lambda frame: model.timeline.get(frame, pos_y),
    lambda frame: model.timeline.get(frame, pos_z),
  )
  mario_path.root_index = path_frames.index(model.selected_frame)
  log.timer.end()

  log.timer.begin('qsteps')
  qstep_frame = model.selected_frame + 1
  num_steps = dcast(int, model.get(qstep_frame, 'gQStepsInfo.numSteps'))
  assert num_steps <= 4

  quarter_steps = []
  for i in range(num_steps):
    quarter_step_value = dcast(dict, model.get(qstep_frame, f'gQStepsInfo.steps[{i}]'))
    quarter_step = cg.QuarterStep()
    quarter_step.intended_pos = cg.vec3(
      quarter_step_value['intendedPos'][0],
      quarter_step_value['intendedPos'][1],
      quarter_step_value['intendedPos'][2],
    )
    quarter_step.result_pos = cg.vec3(
      quarter_step_value['resultPos'][0],
      quarter_step_value['resultPos'][1],
      quarter_step_value['resultPos'][2],
    )
    quarter_steps.append(quarter_step)

  cg.object_path_set_qsteps(
    mario_path, path_frames.index(model.selected_frame), quarter_steps
  )
  log.timer.end()

  return mario_path


def build_scene(
  model: Model,
  viewport: cg.Viewport,
  camera: cg.Camera,
  hidden_surfaces: Set[int],
) -> cg.Scene:
  scene = cg.Scene()
  scene.viewport = viewport
  scene.camera = camera

  def get_field_offset(path: str) -> int:
    return model.game.path(path).edges[-1].value

  log.timer.begin('so')
  memory = model.game.memory
  assert isinstance(memory, AccessibleMemory)
  with model.timeline.request_base(model.selected_frame) as state:
    surface_pool_addr = dcast(Address, state.get('sSurfacePool'))
    if not surface_pool_addr.is_null:
      cg.scene_add_surfaces(
        scene,
        memory.address_to_location(state.slot, surface_pool_addr),
        memory.data_spec['types']['struct']['Surface']['size'],
        dcast(int, state.get('gSurfacesAllocated')),
        get_field_offset,
        list(hidden_surfaces),
      )

    cg.scene_add_objects(
      scene,
      memory.address_to_location(state.slot, state.get_addr('gObjectPool')),
      memory.data_spec['types']['struct']['Object']['size'],
      get_field_offset,
    )
  log.timer.end()

  if model.play_speed <= 0:
    path_frames = range(max(model.selected_frame - 5, 0), model.selected_frame + 61)
  else:
    path_frames = range(max(model.selected_frame - 60, 0), model.selected_frame + 6)
  scene.object_paths = [build_mario_path(model, path_frames)]

  return scene


def render_game(
  model: Model,
  viewport: cg.Viewport,
  camera: cg.Camera,
  wall_hitbox_radius: float,
  hovered_surface: Optional[int] = None,
  hidden_surfaces: Set[int] = set(),
) -> None:
  log.timer.begin('scene')
  scene = build_scene(model, viewport, camera, hidden_surfaces)
  scene.wall_hitbox_radius = wall_hitbox_radius
  scene.hovered_surface = -1 if hovered_surface is None else hovered_surface
  log.timer.end()
  log.timer.begin('render')
  render_scene(scene)
  log.timer.end()


__all__ = ['render_game']
