from typing import *

from ext_modules.graphics import Renderer, init_opengl, Viewport, Camera, Scene, Object, \
  vec2, vec3, vec4

from wafel.model import Model
import wafel.config as config
from wafel.core import VariableParam


_renderer: Optional[Renderer] = None

def get_renderer() -> Renderer:
  global _renderer
  if _renderer is None:
    init_opengl()
    _renderer = Renderer(config.assets_directory)
  return _renderer


def build_scene(model: Model, viewport: Viewport, camera: Camera) -> Scene:
  scene = Scene()
  scene.viewport = viewport
  scene.camera = camera

  with model.timeline[model.selected_frame] as state:
    args = { VariableParam.STATE: state }
    mario_pos = vec3(
      model.variables['mario-pos-x'].get(args),
      model.variables['mario-pos-y'].get(args),
      model.variables['mario-pos-z'].get(args),
    )

  obj = Object()
  obj.pos = mario_pos
  obj.hitbox_height = 150
  obj.hitbox_radius = 37

  scene.objects = [obj]

  return scene


def render_game(model: Model, viewport: Viewport, camera: Camera) -> None:
  scene = build_scene(model, viewport, camera)
  get_renderer().render(scene)


__all__ = ['render_game']
