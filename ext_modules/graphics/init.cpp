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


static Renderer *renderer;


static PyObject *load_gl(PyObject *self, PyObject *args) {
  if (!gladLoadGL()) {
    PyErr_SetString(PyExc_Exception, "Failed to load OpenGL");
    return NULL;
  }

  renderer = new Renderer;

  Py_RETURN_NONE;
}


static void *segmented_to_virtual(sm64::SM64State *st, void *addr) {
  void *result = ((void *)0);
  s32 i = 0;
  for (; (i < 32); (i++)) {
    if (((st->sSegmentTable[i].srcStart <= addr) && (addr < st->sSegmentTable[i].srcEnd))) {
      if ((result != ((void *)0))) {
        fprintf(stderr, "Warning: segmented_to_virtual: Found two segments containing address\n");
        exit(1);
      }
      (result = ((((u8 *)addr) - ((u8 *)st->sSegmentTable[i].srcStart)) + (u8 *)st->sSegmentTable[i].dstStart));
    }
  }
  if ((result == ((void *)0))) {
    (result = addr);
  }
  return result;
}


static u32 get_object_list_from_behavior(u32 *behavior) {
  u32 objectList;

  // If the first behavior command is "begin", then get the object list header
  // from there
  if ((behavior[0] >> 24) == 0) {
    objectList = (behavior[0] >> 16) & 0xFFFF;
  } else {
    objectList = sm64::OBJ_LIST_DEFAULT;
  }

  return objectList;
}

static u32 get_object_list(sm64::Object *object) {
  return get_object_list_from_behavior((u32 *)object->behavior);
}


struct RenderInfo {
  sm64::SM64State *current_state;
};


static sm64::SM64State *read_game_state(PyObject *state_object) {
  if (state_object == NULL) {
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

  Py_DECREF(addr_object);
  Py_DECREF(state_object);
  return (sm64::SM64State *)addr;
}


static bool read_render_info(RenderInfo *info, PyObject *args) {
  PyObject *info_object;
  if (!PyArg_ParseTuple(args, "O", &info_object)) {
    return false;
  }

  info->current_state = read_game_state(PyObject_GetAttrString(info_object, "current_state"));
  if (info->current_state == NULL) {
    return false;
  }
}


static PyObject *render(PyObject *self, PyObject *args) {
  RenderInfo render_info;
  RenderInfo *info = &render_info;

  read_render_info(info, args);


  renderer->clear();
  renderer->set_viewport({{0, 0}, {640, 480}});


  sm64::SM64State *st = info->current_state;

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
    struct sm64::Object *obj = &st->gObjectPool[i];
    if (obj->activeFlags & ACTIVE_FLAG_ACTIVE) {
      renderer->add_object(
        vec3(obj->oPosX, obj->oPosY, obj->oPosZ),
        obj->hitboxHeight);
    }
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
