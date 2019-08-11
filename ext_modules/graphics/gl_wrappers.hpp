#ifndef _GRAPHICS_GL_WRAPPERS_HPP
#define _GRAPHICS_GL_WRAPPERS_HPP


#include "util.hpp"


class Program {
public:
  GLuint name;

  Program(const string &vertex_shader_filename, const string &fragment_shader_filename);
  ~Program();

  void use();

  GLint uniform(const string &name);
  GLint attribute(const string &name);

  void set_uniform(const string &name, const vec4 &value);
  void set_uniform(const string &name, const mat4 &value);
};


class VertexArray {
public:
  GLuint name;

  VertexArray(Program *program);
  ~VertexArray();

  void bind();

  void set(const string &attribute, const vector<vec3> &data);

private:
  Program *program;
  map<string, GLuint> buffers;
};


// class VertexArray {
// public:
//   VertexArray(Program &program);
//   ~VertexArray();



// private:
//   Program &program;
// }


#endif
