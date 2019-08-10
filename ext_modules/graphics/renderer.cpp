#include <cmath>
#include <algorithm>
#include <cstdio>

#include <glad.h>
#include <glm/glm.hpp>
#include <glm/gtc/matrix_transform.hpp>

#include "renderer.hpp"

using namespace std;


Renderer::Renderer() {
  p_surface.program = new Program("assets/shaders/surface.vert", "assets/shaders/surface.frag");
  p_surface.vertex_array = new VertexArray(p_surface.program);
}

void Renderer::clear() {
  p_surface.buffers.pos.clear();
  p_surface.buffers.color.clear();
}

void Renderer::set_viewport(const Viewport &viewport) {
  this->viewport = viewport;
}

void Renderer::set_camera(const Camera &camera) {
  this->camera = camera;
}

void Renderer::add_surface(const Surface &surface) {
  p_surface.buffers.pos.push_back(surface.vertices[0]);
  p_surface.buffers.pos.push_back(surface.vertices[1]);
  p_surface.buffers.pos.push_back(surface.vertices[2]);

  p_surface.buffers.color.push_back(surface.color);
  p_surface.buffers.color.push_back(surface.color);
  p_surface.buffers.color.push_back(surface.color);
}

void Renderer::add_object(vec3 pos, float height) {
  // glColor3f(1, 0, 0);
  // glBegin(GL_LINES);
  // glVertex3f(pos.x, pos.y, pos.z);
  // glVertex3f(pos.x, pos.y + height, pos.z);
  // glEnd();
}

void Renderer::render() {
  glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);


  glViewport(viewport.pos.x, viewport.pos.y, viewport.size.x, viewport.size.y);

  float near = 10;
  float top = near * tanf(camera.fov_y / 2);
  float right = top * viewport.size.x / viewport.size.y;
  mat4 proj_matrix = glm::frustum<float>(-right, right, -top, top, near, 20000);

  mat4 view_matrix(1.0f);
  view_matrix = glm::rotate(view_matrix, glm::pi<float>(), vec3(0, 1, 0));
  view_matrix = glm::rotate(view_matrix, camera.pitch, vec3(1, 0, 0));
  view_matrix = glm::rotate(view_matrix, -camera.yaw, vec3(0, 1, 0));
  view_matrix = glm::translate(view_matrix, -camera.pos);


  glEnable(GL_DEPTH_TEST);
  glDepthFunc(GL_LEQUAL);


  p_surface.program->use();

  p_surface.program->set_uniform("uProjMatrix", proj_matrix);
  p_surface.program->set_uniform("uViewMatrix", view_matrix);

  p_surface.vertex_array->bind();
  p_surface.vertex_array->set("inPos", p_surface.buffers.pos);
  p_surface.vertex_array->set("inColor", p_surface.buffers.color);

  glDrawArrays(GL_TRIANGLES, 0, p_surface.buffers.pos.size());
}
