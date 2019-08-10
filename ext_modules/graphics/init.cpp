#include <cstdio>

#include "util.hpp"

#include <Python.h>
#include <glad.h>
#include <glm/glm.hpp>
namespace sm64 {
  extern "C" {
    #include <libsm64.h>
  }
}

#include "renderer.hpp"

using namespace sm64;


static Renderer *renderer;


static PyObject *load_gl(PyObject *self, PyObject *args) {
  if (!gladLoadGL()) {
    PyErr_SetString(PyExc_Exception, "Failed to load OpenGL");
    return NULL;
  }

  renderer = new Renderer;

  Py_RETURN_NONE;
}


static PyObject *render(PyObject *self, PyObject *args) {
  PyObject *state_object;
  if (!PyArg_ParseTuple(args, "O", &state_object)) {
    return NULL;
  }

  PyObject *addr_object = PyObject_GetAttrString(state_object, "addr");
  if (addr_object == NULL) {
    return NULL;
  }

  long addr = PyLong_AsLong(addr_object);
  if (PyErr_Occurred()) {
    return NULL;
  }

  renderer->clear();
  renderer->set_viewport({{0, 0}, {640, 480}});


  struct SM64State *st = (struct SM64State *)addr;

  f32 *camera_pos = st->D_8033B328.unk0[1];
  f32 camera_pitch = st->D_8033B328.unk4C * 3.14159f / 0x8000;
  f32 camera_yaw = st->D_8033B328.unk4E * 3.14159f / 0x8000;
  f32 camera_fov_y = /*D_8033B234*/ 45 * 3.14159f / 180;

  renderer->set_camera({
    vec3(camera_pos[0], camera_pos[1], camera_pos[2]),
    camera_pitch,
    camera_yaw,
    camera_fov_y,
  });

  for (s32 i = 0; i < st->gSurfacesAllocated; i++) {
    struct sm64::Surface *surface = &st->sSurfacePool[i];

    vec3 color;
    if (surface->normal.y > 0.01f) {
      color = vec3(0.5f, 0.5f, 1.0f);
    } else if (surface->normal.y < -0.01f) {
      color = vec3(1.0f, 0.5f, 0.5f);
    } else if (surface->normal.x < -0.707f || surface->normal.x > 0.707f) {
      color = vec3(0.3f, 0.8f, 0.3f);
    } else {
      color = vec3(0.15f, 0.4f, 0.15f);
    }

    renderer->add_surface({
      {
        vec3(surface->vertex1[0], surface->vertex1[1], surface->vertex1[2]),
        vec3(surface->vertex2[0], surface->vertex2[1], surface->vertex2[2]),
        vec3(surface->vertex3[0], surface->vertex3[1], surface->vertex3[2]),
      },
      color,
    });
  }

  for (s32 i = 0; i < 240; i++) {
    struct Object *obj = &st->gObjectPool[i];
    renderer->add_object(
      vec3(obj->oPosX, obj->oPosY, obj->oPosZ),
      obj->hitboxHeight);
  }

  renderer->render();

  Py_RETURN_NONE;
}


static PyMethodDef method_defs[] = {
  { "load_gl", load_gl, METH_NOARGS, NULL },
  { "render", render, METH_VARARGS, NULL },
  { NULL, NULL, 0, NULL },
};


static PyModuleDef module_def = {
  PyModuleDef_HEAD_INIT,
  "graphics",
  NULL,
  -1,
  method_defs,
};


PyMODINIT_FUNC PyInit_graphics(void) {
  return PyModule_Create(&module_def);
}
