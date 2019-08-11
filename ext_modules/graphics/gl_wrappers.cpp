#include <fstream>
#include <sstream>
#include <cstdio>

#include "gl_wrappers.hpp"


static void compile_shader(GLuint shader, const string &filename, const string &source) {
  const char *source_c_str = source.c_str();
  glShaderSource(shader, 1, &source_c_str, NULL);
  glCompileShader(shader);

  GLint compile_status, info_log_length;
  glGetShaderiv(shader, GL_COMPILE_STATUS, &compile_status);
  glGetShaderiv(shader, GL_INFO_LOG_LENGTH, &info_log_length);

  if (info_log_length != 0) {
    char *info_log = new char[info_log_length];
    glGetShaderInfoLog(shader, info_log_length, NULL, info_log);
    fprintf(stderr, "%s: %s\n", filename.c_str(), info_log);
    delete[] info_log;
  }

  if (compile_status != GL_TRUE) {
    fprintf(stderr, "ERROR: Failed to compile %s\n", filename.c_str());
  }
}

static void compile_shader(GLuint shader, const string &filename) {
  std::ifstream f(filename);
  std::stringstream buffer;
  buffer << f.rdbuf();
  compile_shader(shader, filename, buffer.str().c_str());
}

static void link_program(GLuint program) {
  glLinkProgram(program);

  GLint link_status, info_log_length;
  glGetProgramiv(program, GL_LINK_STATUS, &link_status);
  glGetProgramiv(program, GL_INFO_LOG_LENGTH, &info_log_length);

  if (info_log_length != 0) {
    char *info_log = new char[info_log_length];
    glGetProgramInfoLog(program, info_log_length, NULL, info_log);
    fprintf(stderr, "program: %s\n", info_log);
    delete[] info_log;
  }

  if (link_status != GL_TRUE) {
    fprintf(stderr, "ERROR: Failed to link program\n");
  }
}


Program::Program(const string &vertex_shader_filename, const string &fragment_shader_filename) {
  name = glCreateProgram();

  GLuint vertex_shader = glCreateShader(GL_VERTEX_SHADER);
  compile_shader(vertex_shader, vertex_shader_filename);
  glAttachShader(name, vertex_shader);

  GLuint fragment_shader = glCreateShader(GL_FRAGMENT_SHADER);
  compile_shader(fragment_shader, fragment_shader_filename);
  glAttachShader(name, fragment_shader);

  link_program(name);

  glDetachShader(name, vertex_shader);
  glDeleteShader(vertex_shader);

  glDetachShader(name, fragment_shader);
  glDeleteShader(fragment_shader);
}

Program::~Program() {
  glDeleteProgram(name);
}

void Program::use() {
  glUseProgram(this->name);
}

GLint Program::uniform(const string &name) {
  // TODO: Cache
  return glGetUniformLocation(this->name, name.c_str());
}

GLint Program::attribute(const string &name) {
  // TODO: Cache
  return glGetAttribLocation(this->name, name.c_str());
}

void Program::set_uniform(const string &name, const vec4 &value) {
  use();
  glUniform4f(uniform(name), value.x, value.y, value.z, value.w);
}

void Program::set_uniform(const string &name, const mat4 &value) {
  use();
  glUniformMatrix4fv(uniform(name), 1, GL_FALSE, &value[0][0]);
}


VertexArray::VertexArray(Program *program)
  : program(program)
{
  glGenVertexArrays(1, &name);
}

VertexArray::~VertexArray() {
  for (auto &entry : buffers) {
    glDeleteBuffers(1, &entry.second);
  }

  glDeleteVertexArrays(1, &name);
}

void VertexArray::bind() {
  glBindVertexArray(name);
}

void VertexArray::set(const string &attribute, const vector<vec3> &data) {
  GLuint &buffer = buffers[attribute];
  if (buffer == 0) {
    glGenBuffers(1, &buffer);
    glBindBuffer(GL_ARRAY_BUFFER, buffer);

    bind();
    GLint location = program->attribute(attribute);
    glEnableVertexAttribArray(location);
    glVertexAttribPointer(location, 3, GL_FLOAT, GL_FALSE, 0, 0);
  }

  glBindBuffer(GL_ARRAY_BUFFER, buffer);
  glBufferData(GL_ARRAY_BUFFER, VEC_SIZE(data), data.data(), GL_STATIC_DRAW);
}
