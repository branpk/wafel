from typing import *
import json

from wafel_core import Scene, Viewport, BirdsEyeCamera, RotateCamera, QuarterStep, VizRenderData

from wafel.model import Model
from wafel.util import *


scenes: List[Scene] = []
viz_scenes: List[VizRenderData] = []

def take_scenes() -> Tuple[List[Scene], List[VizRenderData]]:
  global scenes, viz_scenes
  result = (scenes, viz_scenes)
  scenes = []
  viz_scenes = []
  return result


def get_viz_config(
  model: Model,
  viewport: Viewport,
  camera: RotateCamera,
) -> dict:
  config = {
    'screen_top_left': [int(viewport.x), int(viewport.y)],
    'screen_size': [int(viewport.width), int(viewport.height)],
    'camera': {
      'LookAt': {
        'pos': camera.pos,
        'focus': camera.target,
        'roll': 0, # TODO: Roll (and fov?)
      }
    },
  }
  config.update(model.viz_config)
  return config


def render_game(
  model: Model,
  viewport: Viewport,
  camera: Union[BirdsEyeCamera, RotateCamera],
  show_camera_target: bool,
  wall_hitbox_radius: float,
  hovered_surface: Optional[int] = None,
  hidden_surfaces: Set[int] = set(),
) -> None:
  if model.viz_enabled and isinstance(camera, RotateCamera):
    viz_config = get_viz_config(model, viewport, camera)
    viz_config_json = json.dumps(viz_config)
    viz_scene = model.pipeline.render(model.selected_frame, viz_config_json)
    if viz_scene is not None:
      viz_scenes.append(viz_scene)
      return

  scene = Scene()
  scene.viewport = viewport
  scene.camera = camera
  scene.show_camera_target = show_camera_target

  model.pipeline.read_surfaces_to_scene(scene, model.selected_frame)
  scene.wall_hitbox_radius = wall_hitbox_radius
  scene.hovered_surface = hovered_surface
  scene.hidden_surfaces = hidden_surfaces

  model.pipeline.read_objects_to_scene(scene, model.selected_frame)

  if model.play_speed <= 0 or not model.playback_mode:
    path_frames = range(max(model.selected_frame - 5, 0), model.selected_frame + 61)
  else:
    path_frames = range(max(model.selected_frame - 60, 0), model.selected_frame + 6)
  mario_path = model.pipeline.read_mario_path(path_frames.start, path_frames.stop)
  mario_path.root_index = path_frames.index(model.selected_frame)

  log.timer.begin('qsteps')
  qstep_frame = model.selected_frame + 1
  num_steps = dcast(int, model.get(qstep_frame, 'gQStepsInfo.numSteps'))

  quarter_steps = []
  for i in range(num_steps):
    quarter_step_value = dcast(dict, model.get(qstep_frame, f'gQStepsInfo.steps[{i}]'))
    quarter_step = QuarterStep()
    quarter_step.intended_pos = quarter_step_value['intendedPos']
    quarter_step.result_pos = quarter_step_value['resultPos']
    quarter_steps.append(quarter_step)

  mario_path.set_quarter_steps(path_frames.index(qstep_frame) - 1, quarter_steps)
  log.timer.end()

  scene.object_paths = [mario_path]

  scenes.append(scene)


__all__ = [
  'get_scenes',
  'render_game',
]
