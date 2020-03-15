#include <cstdio>
#include <algorithm>

#include <pybind11/pybind11.h>
#include <pybind11/stl.h>
#include <glad.h>
#include <glm/glm.hpp>
#include <glm/gtc/matrix_transform.hpp>

#include "renderer.hpp"
#include "util.hpp"
#include "sm64.hpp"

namespace py = pybind11;

using sm64::s8;
using sm64::s16;
using sm64::s32;
using sm64::s64;
using sm64::u8;
using sm64::u16;
using sm64::u32;
using sm64::u64;
using sm64::f32;
using sm64::f64;

typedef u64 uptr; // integer at least the size of a pointer, for pybind11 conversions


#define VEC3F_TO_VEC3(v) (vec3((v)[0], (v)[1], (v)[2]))


static void init_opengl() {
  static bool loaded_gl = false;

  if (!loaded_gl) {
    if (!gladLoadGL()) {
      throw std::runtime_error("Failed to load OpenGL");
    }
    loaded_gl = true;
  }
}


PYBIND11_MODULE(graphics, m) {
  m.def("init_opengl", init_opengl);

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
    .def_readwrite("camera", &Scene::camera)
    .def_readwrite("surfaces", &Scene::surfaces)
    .def_readwrite("objects", &Scene::objects)
    .def_readwrite("object_paths", &Scene::object_paths);
}
