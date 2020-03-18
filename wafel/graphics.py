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


def build_mario_path(model: Model, path_frames: range) -> cg.ObjectPath:
  # 87 -> 96 -> 36
  # 112
  # with model.timeline[model.selected_frame] as state:
  #   log.timer.begin('test')
  #   for _ in range(3000):
  #     model.variables['mario-pos-x'].get(state)
  #     model.variables['mario-pos-y'].get(state)
  #     model.variables['mario-pos-z'].get(state)
  #   log.timer.end()

  mario_path_nodes = []
  for frame in path_frames:
    path_node = cg.ObjectPathNode()
    path_node.pos = cg.vec3(
      model.timeline.get_cached(frame, model.variables['mario-pos-x']),
      model.timeline.get_cached(frame, model.variables['mario-pos-y']),
      model.timeline.get_cached(frame, model.variables['mario-pos-z']),
    )
    mario_path_nodes.append(path_node)

  with model.timeline[model.selected_frame + 1] as state:
    def get(path: str) -> Any:
      return DataPath.compile(model.lib, path).get(state)

    num_steps = get('$state.gQStepsInfo.numSteps')
    assert num_steps <= 4

    quarter_steps = []
    for i in range(num_steps):
      quarter_step = cg.QuarterStep()
      quarter_step.intended_pos = cg.vec3(
        get(f'$state.gQStepsInfo.steps[{i}].intendedPos[0]'),
        get(f'$state.gQStepsInfo.steps[{i}].intendedPos[1]'),
        get(f'$state.gQStepsInfo.steps[{i}].intendedPos[2]'),
      )
      quarter_step.result_pos = cg.vec3(
        get(f'$state.gQStepsInfo.steps[{i}].resultPos[0]'),
        get(f'$state.gQStepsInfo.steps[{i}].resultPos[1]'),
        get(f'$state.gQStepsInfo.steps[{i}].resultPos[2]'),
      )
      quarter_steps.append(quarter_step)

    root_node = mario_path_nodes[path_frames.index(model.selected_frame)]
    root_node.quarter_steps = quarter_steps

  mario_path = cg.ObjectPath()
  mario_path.nodes = mario_path_nodes
  mario_path.root_index = path_frames.index(model.selected_frame)

  return mario_path


def build_scene(model: Model, viewport: cg.Viewport, camera: cg.Camera) -> cg.Scene:
  scene = cg.Scene()
  scene.viewport = viewport
  scene.camera = camera

  def get_field_offset(path: str) -> int:
    # TODO: Less hacky way to do this?
    data_path = DataPath.compile(model.lib, path)
    offset = data_path.addr_path.path[-1].value
    return dcast(int, offset)

  with model.timeline[model.selected_frame] as state:
    surface_pool_addr = DataPath.compile(model.lib, '$state.sSurfacePool').get(state)
    assert isinstance(surface_pool_addr, RelativeAddr)
    cg.scene_add_surfaces(
      scene,
      state.slot.relative_to_addr(surface_pool_addr),
      model.lib.spec['types']['struct']['Surface']['size'],
      DataPath.compile(model.lib, '$state.gSurfacesAllocated').get(state),
      get_field_offset,
    )

    cg.scene_add_objects(
      scene,
      DataPath.compile(model.lib, '$state.gObjectPool').get_addr(state),
      model.lib.spec['types']['struct']['Object']['size'],
      get_field_offset,
    )

  path_frames = range(max(model.selected_frame - 5, 0), model.selected_frame + 61)
  scene.object_paths = [build_mario_path(model, path_frames)]

  return scene


def render_game(
  model: Model,
  viewport: cg.Viewport,
  camera: cg.Camera,
  wall_hitbox_radius: float,
) -> None:
  scene = build_scene(model, viewport, camera)
  scene.wall_hitbox_radius = wall_hitbox_radius
  get_renderer().render(scene)


__all__ = ['render_game']
