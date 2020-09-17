from typing import *

from ext_modules.core import Scene, Viewport, BirdsEyeCamera, RotateCamera, QuarterStep

from wafel.model import Model
import wafel.config as config
from wafel.util import *


scenes: List[Scene] = []

def take_scenes() -> List[Scene]:
  global scenes
  result = scenes
  scenes = []
  return result


def render_game(
  model: Model,
  viewport: Viewport,
  camera: Union[BirdsEyeCamera, RotateCamera],
  show_camera_target: bool,
  wall_hitbox_radius: float,
  hovered_surface: Optional[int] = None,
  hidden_surfaces: Set[int] = set(),
) -> None:
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
