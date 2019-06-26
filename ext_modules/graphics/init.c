#include <stdio.h>

#include <Python.h>
#include <glad.h>
#include <sm64plus.h>


static PyObject *load_gl(PyObject *self, PyObject *args) {
  if (!gladLoadGL()) {
    PyErr_SetString(PyExc_Exception, "Failed to load OpenGL");
    return NULL;
  }
  Py_RETURN_NONE;
}


static PyObject *render(PyObject *self, PyObject *args) {
  glClearColor(0.5f, 0, 0, 1);
  glClear(GL_COLOR_BUFFER_BIT);

  Py_RETURN_NONE;
}


static PyMethodDef method_defs[] = {
  { "load_gl", load_gl, METH_NOARGS, NULL },
  { "render", render, METH_NOARGS, NULL },
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
