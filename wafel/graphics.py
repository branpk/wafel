from typing import *

import ext_modules.graphics as cg

from wafel.model import Model
import wafel.config as config
from wafel.core import DataPath, Object, RelativeAddr
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
  log.timer.begin('nodes')
  mario_path_nodes = []
  for frame in path_frames:
    path_node = cg.ObjectPathNode()
    path_node.pos = cg.vec3(
      model.timeline.get_cached(frame, model.variables['mario-pos-x']),
      model.timeline.get_cached(frame, model.variables['mario-pos-y']),
      model.timeline.get_cached(frame, model.variables['mario-pos-z']),
    )
    mario_path_nodes.append(path_node)
  log.timer.end()

  log.timer.begin('qsteps')
  with model.timeline[model.selected_frame + 1] as state:
    num_steps = dcast(int, DataPath.compile(model.lib, '$state.gQStepsInfo.numSteps').get(state))
    assert num_steps <= 4

    quarter_steps = []
    for i in range(num_steps):
      quarter_step_value = \
        dcast(dict, DataPath.compile(model.lib, f'$state.gQStepsInfo.steps[{i}]').get(state))
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

    root_node = mario_path_nodes[path_frames.index(model.selected_frame)]
    root_node.quarter_steps = quarter_steps
  log.timer.end()

  mario_path = cg.ObjectPath()
  mario_path.nodes = mario_path_nodes
  mario_path.root_index = path_frames.index(model.selected_frame)

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
    # TODO: Less hacky way to do this?
    data_path = DataPath.compile(model.lib, path)
    offset = data_path.addr_path.path[-1].value
    return dcast(int, offset)

  log.timer.begin('so')
  with model.timeline[model.selected_frame] as state:
    surface_pool_addr = DataPath.compile(model.lib, '$state.sSurfacePool').get(state)
    assert isinstance(surface_pool_addr, RelativeAddr)
    cg.scene_add_surfaces(
      scene,
      state.slot.relative_to_addr(surface_pool_addr),
      model.lib.spec['types']['struct']['Surface']['size'],
      DataPath.compile(model.lib, '$state.gSurfacesAllocated').get(state),
      get_field_offset,
      list(hidden_surfaces),
    )

    cg.scene_add_objects(
      scene,
      DataPath.compile(model.lib, '$state.gObjectPool').get_addr(state),
      model.lib.spec['types']['struct']['Object']['size'],
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
