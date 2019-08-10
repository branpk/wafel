#include <stdio.h>

#include <Python.h>
#include <glad.h>
#include <libsm64.h>


static PyObject *load_gl(PyObject *self, PyObject *args) {
  if (!gladLoadGL()) {
    PyErr_SetString(PyExc_Exception, "Failed to load OpenGL");
    return NULL;
  }

  glEnable(GL_DEPTH_TEST);
  glDepthFunc(GL_LEQUAL);

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

  struct SM64State *st = (struct SM64State *)addr;

  int width = 640;
  int height = 480;

  glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

  f32 *cameraPos = st->D_8033B328.unk0[1];
  f32 cameraPitch = st->D_8033B328.unk4C * 3.14159f / 0x8000;
  f32 cameraYaw = st->D_8033B328.unk4E * 3.14159f / 0x8000;
  f32 cameraFovY = /*D_8033B234*/ 45 * 3.14159f / 180;

  f32 nearV = 10;
  f32 top = nearV * tanf(cameraFovY / 2);
  f32 right = top * width / height;

  glMatrixMode(GL_PROJECTION);
  glLoadIdentity();
  glFrustum(-right, right, -top, top, nearV, 20000);

  glMatrixMode(GL_MODELVIEW);
  glLoadIdentity();
  glRotatef(180, 0, 1, 0);
  glRotatef(cameraPitch * 180 / 3.14159f, 1, 0, 0);
  glRotatef(-cameraYaw * 180 / 3.14159f, 0, 1, 0);
  glTranslatef(-cameraPos[0], -cameraPos[1], -cameraPos[2]);

  glColor3f(0.7, 0.7, 0.7);
  for (s32 i = 0; i < st->gSurfacesAllocated; i++) {
    struct Surface *surface = &st->sSurfacePool[i];

    glBegin(GL_TRIANGLES);
    glVertex3f(surface->vertex1[0], surface->vertex1[1], surface->vertex1[2]);
    glVertex3f(surface->vertex2[0], surface->vertex2[1], surface->vertex2[2]);
    glVertex3f(surface->vertex3[0], surface->vertex3[1], surface->vertex3[2]);
    glEnd();
  }

  glColor3f(0, 0, 0);
  for (s32 i = 0; i < st->gSurfacesAllocated; i++) {
    struct Surface *surface = &st->sSurfacePool[i];

    glBegin(GL_LINE_LOOP);
    glVertex3f(surface->vertex1[0], surface->vertex1[1], surface->vertex1[2]);
    glVertex3f(surface->vertex2[0], surface->vertex2[1], surface->vertex2[2]);
    glVertex3f(surface->vertex3[0], surface->vertex3[1], surface->vertex3[2]);
    glEnd();
  }

  glColor3f(1, 0, 0);
  glBegin(GL_LINES);
  glVertex3f(st->gMarioState->pos[0], st->gMarioState->pos[1], st->gMarioState->pos[2]);
  glVertex3f(st->gMarioState->pos[0], st->gMarioState->pos[1] + 160, st->gMarioState->pos[2]);
  glEnd();

  glColor3f(1, 0, 0);
  glBegin(GL_LINES);
  for (s32 i = 0; i < 240; i++) {
    struct Object *obj = &st->gObjectPool[i];
    if (obj->activeFlags & ACTIVE_FLAG_ACTIVE) {
      glVertex3f(obj->oPosX, obj->oPosY, obj->oPosZ);
      glVertex3f(obj->oPosX, obj->oPosY + obj->hitboxHeight, obj->oPosZ);
    }
  }
  glEnd();

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
