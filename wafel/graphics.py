from typing import *

from ext_modules.graphics import Renderer, init_opengl, Viewport, Camera, Scene, Object, \
  vec2, vec3, vec4, scene_add_surfaces, scene_add_objects

from wafel.model import Model
import wafel.config as config
from wafel.core import VariableParam, DataPath, Object
from wafel.util import *


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

  # TODO: Use for surfaces as well?
  def get_field_offset(path: str) -> int:
    # TODO: Less hacky way to do this?
    data_path = DataPath.parse(model.lib, path)
    offset = data_path.offset # type: ignore
    return dcast(int, offset)

  with model.timeline[model.selected_frame] as state:
    args = { VariableParam.STATE: state }
    scene_add_surfaces(
      scene,
      DataPath.parse(model.lib, '$state.sSurfacePool').get(args),
      model.lib.spec['types']['struct']['Surface']['size'],
      DataPath.parse(model.lib, '$state.gSurfacesAllocated').get(args),
      get_field_offset,
    )
    scene_add_objects(
      scene,
      DataPath.parse(model.lib, '$state.gObjectPool').get_addr(args),
      model.lib.spec['types']['struct']['Object']['size'],
      get_field_offset,
    )

  return scene


def render_game(model: Model, viewport: Viewport, camera: Camera) -> None:
  scene = build_scene(model, viewport, camera)
  get_renderer().render(scene)


__all__ = ['render_game']
