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


static PyObject *new_renderer(PyObject *self, PyObject *args) {
  static bool loaded_gl = false;

  if (!loaded_gl) {
    if (!gladLoadGL()) {
      PyErr_SetString(PyExc_Exception, "Failed to load OpenGL");
      return NULL;
    }
    loaded_gl = true;
  }

  Renderer *renderer = new Renderer;

  return PyLong_FromVoidPtr((void *)renderer);
}


static PyObject *delete_renderer(PyObject *self, PyObject *args) {
  PyObject *renderer_object;
  if (!PyArg_ParseTuple(args, "O", &renderer_object)) {
    return NULL;
  }

  Renderer *renderer = (Renderer *)PyLong_AsVoidPtr(renderer_object);
  if (PyErr_Occurred()) {
    return NULL;
  }

  delete renderer;

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


struct GameState {
  int frame;
  sm64::SM64State *base;
  sm64::SM64State *data;

  template<typename T>
  T *from_base(T *addr) {
    return (T *)((char *)addr - (char *)base + (char *)data);
  }
};


struct RenderInfo {
  Camera camera;
  GameState current_state;
  vector<GameState> path_states;
};


static int read_int(int *result, PyObject *int_object) {
  if (int_object == NULL) {
    return false;
  }

  *result = (int)PyLong_AsLong(int_object);
  if (PyErr_Occurred()) {
    return false;
  }

  Py_DECREF(int_object);
  return true;
}


static bool read_float(float *result, PyObject *float_object) {
  if (float_object == NULL) {
    return false;
  }

  *result = (float)PyFloat_AsDouble(float_object);
  if (PyErr_Occurred()) {
    return false;
  }

  Py_DECREF(float_object);
  return true;
}


static bool read_vec3(vec3 *result, PyObject *vec_object) {
  if (vec_object == NULL) {
    return false;
  }

  for (int i = 0; i < 3; i++) {
    PyObject *index = PyLong_FromLong(i);
    if (index == NULL) {
      return false;
    }

    if (!read_float(&(*result)[i], PyObject_GetItem(vec_object, index))) {
      return false;
    }

    Py_DECREF(index);
  }

  Py_DECREF(vec_object);
  return true;
}

static bool read_camera(Camera *camera, PyObject *camera_object) {
  if (camera_object == NULL) {
    return false;
  }

  PyObject *mode_object = PyObject_GetAttrString(camera_object, "mode");
  if (mode_object == NULL) {
    return false;
  }
  PyObject *mode_int_object = PyObject_GetAttrString(mode_object, "value");
  if (mode_int_object == NULL) {
    return false;
  }
  camera->mode = (CameraMode)PyLong_AsLong(mode_int_object);
  if (PyErr_Occurred()) {
    return false;
  }
  Py_DECREF(mode_int_object);
  Py_DECREF(mode_object);

  switch (camera->mode) {
    case CameraMode::ROTATE: {
      if (!read_vec3(&camera->rotate_camera.pos, PyObject_GetAttrString(camera_object, "pos")) ||
        !read_float(&camera->rotate_camera.pitch, PyObject_GetAttrString(camera_object, "pitch")) ||
        !read_float(&camera->rotate_camera.yaw, PyObject_GetAttrString(camera_object, "yaw")) ||
        !read_float(&camera->rotate_camera.fov_y, PyObject_GetAttrString(camera_object, "fov_y")))
      {
        return false;
      }
      break;
    }
    case CameraMode::BIRDS_EYE: {
      if (!read_vec3(&camera->birds_eye_camera.pos, PyObject_GetAttrString(camera_object, "pos")) ||
        !read_float(&camera->birds_eye_camera.span_y, PyObject_GetAttrString(camera_object, "span_y")))
      {
        return false;
      }
      break;
    }
  }

  Py_DECREF(camera_object);
  return true;
}


static bool read_game_state(GameState *state, PyObject *state_object) {
  if (state_object == NULL) {
    return false;
  }

  if (!read_int(&state->frame, PyObject_GetAttrString(state_object, "frame"))) {
    return false;
  }

  PyObject *addr_object = PyObject_GetAttrString(state_object, "addr");
  if (addr_object == NULL) {
    return false;
  }
  state->data = (sm64::SM64State *)PyLong_AsVoidPtr(addr_object);
  if (PyErr_Occurred()) {
    return false;
  }

  PyObject *base_addr_object = PyObject_GetAttrString(state_object, "base_addr");
  if (base_addr_object == NULL) {
    return false;
  }
  state->base = (sm64::SM64State *)PyLong_AsVoidPtr(base_addr_object);
  if (PyErr_Occurred()) {
    return false;
  }

  Py_DECREF(addr_object);
  Py_DECREF(state_object);
  return true;
}


static bool read_game_state_list(vector<GameState> *states, PyObject *states_object) {
  if (states_object == NULL) {
    return false;
  }

  *states = vector<GameState>(30);

  size_t length = PyObject_Length(states_object);
  for (size_t i = 0; i < length; i++) {
    PyObject *index = PyLong_FromLong(i);
    if (index == NULL) {
      return false;
    }

    if (!read_game_state(&(*states)[i], PyObject_GetItem(states_object, index))) {
      return false;
    }

    Py_DECREF(index);
  }

  Py_DECREF(states_object);
  return true;
}


static bool read_render_args(Renderer **renderer, RenderInfo *info, PyObject *args) {
  PyObject *renderer_object, *info_object;
  if (!PyArg_ParseTuple(args, "OO", &renderer_object, &info_object)) {
    return false;
  }

  *renderer = (Renderer *)PyLong_AsVoidPtr(renderer_object);
  if (PyErr_Occurred()) {
    return false;
  }

  if (!read_camera(&info->camera, PyObject_GetAttrString(info_object, "camera"))) {
    return false;
  }

  if (!read_game_state(&info->current_state, PyObject_GetAttrString(info_object, "current_state"))) {
    return false;
  }

  if (!read_game_state_list(&info->path_states, PyObject_GetAttrString(info_object, "path_states"))) {
    return false;
  }

  return true;
}


static PyObject *render(PyObject *self, PyObject *args) {
  Renderer *renderer;
  RenderInfo render_info;
  RenderInfo *info = &render_info;

  if (!read_render_args(&renderer, info, args)) {
    return NULL;
  }


  Viewport viewport = {{0, 0}, {640, 480}};
  Scene scene;


  GameState st = info->current_state;

  // f32 *camera_pos = st->D_8033B328.unk0[1];
  // f32 camera_pitch = st->D_8033B328.unk4C * 3.14159f / 0x8000;
  // f32 camera_yaw = st->D_8033B328.unk4E * 3.14159f / 0x8000;
  // f32 camera_fov_y = /*D_8033B234*/ 45 * 3.14159f / 180;

  scene.camera = info->camera;


  for (s32 i = 0; i < st.data->gSurfacesAllocated; i++) {
    struct sm64::Surface *surface = &st.from_base(st.data->sSurfacePool)[i];

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

    scene.surfaces.push_back({
      {
        vec3(surface->vertex1[0], surface->vertex1[1], surface->vertex1[2]),
        vec3(surface->vertex2[0], surface->vertex2[1], surface->vertex2[2]),
        vec3(surface->vertex3[0], surface->vertex3[1], surface->vertex3[2]),
      },
      color,
    });
  }

  for (s32 i = 0; i < 240; i++) {
    sm64::Object *obj = &st.data->gObjectPool[i];
    if (obj->activeFlags & ACTIVE_FLAG_ACTIVE) {
      scene.objects.push_back({
        vec3(obj->oPosX, obj->oPosY, obj->oPosZ),
        obj->hitboxHeight,
      });
    }
  }

  vector<vec3> mario_path;
  for (GameState path_st : info->path_states) {
    sm64::MarioState *m = path_st.from_base(path_st.data->gMarioState);
    mario_path.push_back(vec3(m->pos[0], m->pos[1], m->pos[2]));
  }
  scene.object_paths.push_back({
    mario_path,
  });

  renderer->render(viewport, scene);

  Py_RETURN_NONE;
}


static PyMethodDef method_defs[] = {
  { "new_renderer", new_renderer, METH_NOARGS, NULL },
  { "delete_renderer", delete_renderer, METH_VARARGS, NULL },
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
