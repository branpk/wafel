#include <cstdio>
#include <algorithm>
#include <cstdint>

#include <pybind11/pybind11.h>
#include <pybind11/stl.h>
#include <pybind11/functional.h>
#include <glad.h>
#include <glm/glm.hpp>
#include <glm/gtc/matrix_transform.hpp>

#include "renderer.hpp"
#include "util.hpp"
#include "gfx_rendering_api.h"
#include "scene.hpp"

namespace py = pybind11;


typedef int8_t s8;
typedef int16_t s16;
typedef int32_t s32;
typedef int64_t s64;
typedef uint8_t u8;
typedef uint16_t u16;
typedef uint32_t u32;
typedef uint64_t u64;
typedef float f32;
typedef double f64;

#define ACTIVE_FLAG_ACTIVE                 (1 <<  0) // 0x0001


static void init_opengl() {
  static bool loaded_gl = false;

  if (!loaded_gl) {
    if (!gladLoadGL()) {
      throw std::runtime_error("Failed to load OpenGL");
    }
    loaded_gl = true;
  }
}


static void scene_add_surfaces(
  Scene &scene,
  uintptr_t surface_pool_ptr,
  size_t surface_size,
  s32 num_surfaces,
  function<size_t(const string &)> get_field_offset)
{
  size_t f_normal = get_field_offset("$state.sSurfacePool[].normal");
  size_t f_vertex1 = get_field_offset("$state.sSurfacePool[].vertex1");
  size_t f_vertex2 = get_field_offset("$state.sSurfacePool[].vertex2");
  size_t f_vertex3 = get_field_offset("$state.sSurfacePool[].vertex3");

  for (s32 i = 0; i < num_surfaces; i++) {
    uintptr_t surface_ptr = surface_pool_ptr + i * surface_size;

    f32 *normal = (f32 *) (surface_ptr + f_normal);
    s16 *vertex1 = (s16 *) (surface_ptr + f_vertex1);
    s16 *vertex2 = (s16 *) (surface_ptr + f_vertex2);
    s16 *vertex3 = (s16 *) (surface_ptr + f_vertex3);

    SurfaceType type;
    if (normal[1] > 0.01) {
      type = SurfaceType::FLOOR;
    } else if (normal[1] < -0.01) {
      type = SurfaceType::CEILING;
    } else if (normal[0] < -0.707 || normal[0] > 0.707) {
      type = SurfaceType::WALL_X_PROJ;
    } else {
      type = SurfaceType::WALL_Z_PROJ;
    }

    scene.surfaces.push_back({
      type,
      {
        vec3(vertex1[0], vertex1[1], vertex1[2]),
        vec3(vertex2[0], vertex2[1], vertex2[2]),
        vec3(vertex3[0], vertex3[1], vertex3[2]),
      },
      vec3(normal[0], normal[1], normal[2]),
    });
  }
}


static void scene_add_objects(
  Scene &scene,
  uintptr_t object_pool_ptr,
  size_t object_size,
  function<size_t(const string &)> get_field_offset)
{
  size_t f_active_flags = get_field_offset("$object.activeFlags");
  size_t f_pos_x = get_field_offset("$object.oPosX");
  size_t f_pos_y = get_field_offset("$object.oPosY");
  size_t f_pos_z = get_field_offset("$object.oPosZ");
  size_t f_hitbox_height = get_field_offset("$object.hitboxHeight");
  size_t f_hitbox_radius = get_field_offset("$object.hitboxRadius");

  for (s32 i = 0; i < 240; i++) {
    uintptr_t object_ptr = object_pool_ptr + i * object_size;
    s16 active_flags = *(s16 *) (object_ptr + f_active_flags);
    if (active_flags & ACTIVE_FLAG_ACTIVE) {
      scene.objects.push_back({
        vec3(
          *(f32 *) (object_ptr + f_pos_x),
          *(f32 *) (object_ptr + f_pos_y),
          *(f32 *) (object_ptr + f_pos_z)),
        *(f32 *) (object_ptr + f_hitbox_height),
        *(f32 *) (object_ptr + f_hitbox_radius),
      });
    }
  }
}


typedef void (*p_sm64_update_and_render)(
  uint32_t width,
  uint32_t height,
  struct GfxRenderingAPI *rendering_api);

extern "C" struct GfxRenderingAPI gfx_opengl_api;
extern "C" struct {
    uint32_t x;
    uint32_t y;
    uint32_t width;
    uint32_t height;
} gfx_viewport;
extern "C" void gfx_opengl_end_frame(void);

static void update_and_render(Viewport viewport, uintptr_t update_and_render_fn) {
  p_sm64_update_and_render sm64_update_and_render =
    (p_sm64_update_and_render) update_and_render_fn;

  init_opengl();

  gfx_viewport.x = viewport.pos.x;
  gfx_viewport.y = viewport.pos.y;
  gfx_viewport.width = viewport.size.x;
  gfx_viewport.height = viewport.size.y;

  sm64_update_and_render(viewport.size.x, viewport.size.y, &gfx_opengl_api);
  gfx_opengl_end_frame();
}


PYBIND11_MODULE(graphics, m) {
  m.def("init_opengl", init_opengl);
  m.def("scene_add_surfaces", scene_add_surfaces);
  m.def("scene_add_objects", scene_add_objects);
  m.def("update_and_render", update_and_render);

  // TODO: Generate these automatically? Could create .pyi then too

  py::class_<Renderer>(m, "Renderer")
    .def(py::init<const string &>())
    .def("render", &Renderer::render);

  py::class_<ivec2>(m, "ivec2")
    .def(py::init<>())
    .def(py::init<glm::i32, glm::i32>())
    .def_readwrite("x", &ivec2::x)
    .def_readwrite("y", &ivec2::y)
    .def("__repr__",
      [](const ivec2 &v) {
        return "ivec2(" + std::to_string(v.x) + ", " + std::to_string(v.y) + ")";
      });

  py::class_<vec2>(m, "vec2")
    .def(py::init<>())
    .def(py::init<float, float>())
    .def_readwrite("x", &vec2::x)
    .def_readwrite("y", &vec2::y)
    .def("__repr__",
      [](const vec2 &v) {
        return "vec2(" + std::to_string(v.x) + ", " + std::to_string(v.y) + ")";
      });

  py::class_<vec3>(m, "vec3")
    .def(py::init<>())
    .def(py::init<float, float, float>())
    .def_readwrite("x", &vec3::x)
    .def_readwrite("y", &vec3::y)
    .def_readwrite("z", &vec3::z)
    .def("__repr__",
      [](const vec3 &v) {
        return "vec3(" +
          std::to_string(v.x) + ", " +
          std::to_string(v.y) + ", " +
          std::to_string(v.z) + ")";
      });

  py::class_<vec4>(m, "vec4")
    .def(py::init<>())
    .def(py::init<float, float, float, float>())
    .def_readwrite("x", &vec4::x)
    .def_readwrite("y", &vec4::y)
    .def_readwrite("z", &vec4::z)
    .def_readwrite("w", &vec4::w)
    .def("__repr__",
      [](const vec4 &v) {
        return "vec4(" +
          std::to_string(v.x) + ", " +
          std::to_string(v.y) + ", " +
          std::to_string(v.z) + ", " +
          std::to_string(v.w) + ")";
      });

  py::class_<Viewport>(m, "Viewport")
    .def(py::init<>())
    .def_readwrite("pos", &Viewport::pos)
    .def_readwrite("size", &Viewport::size);

  py::enum_<CameraMode>(m, "CameraMode")
    .value("ROTATE", CameraMode::ROTATE)
    .value("BIRDS_EYE", CameraMode::BIRDS_EYE);

  py::class_<RotateCamera>(m, "RotateCamera")
    .def(py::init<>())
    .def_readwrite("pos", &RotateCamera::pos)
    .def_readwrite("pitch", &RotateCamera::pitch)
    .def_readwrite("yaw", &RotateCamera::yaw)
    .def_readwrite("fov_y", &RotateCamera::fov_y);

  py::class_<BirdsEyeCamera>(m, "BirdsEyeCamera")
    .def(py::init<>())
    .def_readwrite("pos", &BirdsEyeCamera::pos)
    .def_readwrite("span_y", &BirdsEyeCamera::span_y);

  py::class_<Camera>(m, "Camera")
    .def(py::init<>())
    .def(py::init(
      [](const RotateCamera &rotate_camera) {
        Camera camera;
        camera.mode = CameraMode::ROTATE;
        camera.rotate_camera = rotate_camera;
        return camera;
      }))
    .def(py::init(
      [](const BirdsEyeCamera &birds_eye_camera) {
        Camera camera;
        camera.mode = CameraMode::BIRDS_EYE;
        camera.birds_eye_camera = birds_eye_camera;
        return camera;
      }))
    .def_readwrite("mode", &Camera::mode)
    .def_readwrite("rotate_camera", &Camera::rotate_camera)
    .def_readwrite("birds_eye_camera", &Camera::birds_eye_camera);

  py::enum_<SurfaceType>(m, "SurfaceType")
    .value("FLOOR", SurfaceType::FLOOR)
    .value("CEILING", SurfaceType::CEILING)
    .value("WALL_X_PROJ", SurfaceType::WALL_X_PROJ)
    .value("WALL_Z_PROJ", SurfaceType::WALL_Z_PROJ);

  py::class_<Surface>(m, "Surface")
    .def(py::init<>())
    .def_readwrite("type", &Surface::type)
    .def_readwrite("vertices", &Surface::vertices)
    .def_readwrite("normal", &Surface::normal);

  py::class_<Object>(m, "Object")
    .def(py::init<>())
    .def_readwrite("pos", &Object::pos)
    .def_readwrite("hitbox_height", &Object::hitbox_height)
    .def_readwrite("hitbox_radius", &Object::hitbox_radius);

  py::class_<QuarterStep>(m, "QuarterStep")
    .def(py::init<>())
    .def_readwrite("intended_pos", &QuarterStep::intended_pos)
    .def_readwrite("result_pos", &QuarterStep::result_pos);

  py::class_<ObjectPathNode>(m, "ObjectPathNode")
    .def(py::init<>())
    .def_readwrite("pos", &ObjectPathNode::pos)
    .def_readwrite("quarter_steps", &ObjectPathNode::quarter_steps);

  py::class_<ObjectPath>(m, "ObjectPath")
    .def(py::init<>())
    .def_readwrite("nodes", &ObjectPath::nodes)
    .def_readwrite("root_index", &ObjectPath::root_index);

  py::class_<Scene>(m, "Scene")
    .def(py::init<>())
    .def_readwrite("viewport", &Scene::viewport)
    .def_readwrite("camera", &Scene::camera)
    .def_readwrite("surfaces", &Scene::surfaces)
    .def_readwrite("objects", &Scene::objects)
    .def_readwrite("object_paths", &Scene::object_paths);
}
