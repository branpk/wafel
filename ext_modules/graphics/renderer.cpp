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

  p_object.program = new Program("assets/shaders/transform.vert", "assets/shaders/uniform_color.frag");
  p_object.vertex_array = new VertexArray(p_object.program);
}

void Renderer::clear() {
  p_surface.buffers.pos.clear();
  p_surface.buffers.color.clear();

  p_object.buffers.pos.clear();
}

void Renderer::set_viewport(const Viewport &viewport) {
  this->viewport = viewport;
  this->viewport.size.x = max(this->viewport.size.x, 1);
  this->viewport.size.y = max(this->viewport.size.y, 1);
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
  p_object.buffers.pos.push_back(pos);
  p_object.buffers.pos.push_back(pos + vec3(0, height, 0));
}

void Renderer::render() {
  glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);


  glViewport(viewport.pos.x, viewport.pos.y, viewport.size.x, viewport.size.y);


  mat4 proj_matrix, view_matrix;



  switch (camera.mode) {
    case CameraMode::ROTATE: {
      RotateCamera &cam = camera.rotate_camera;

      float near = 10;
      float top = near * tanf(cam.fov_y / 2);
      float right = top * viewport.size.x / viewport.size.y;
      proj_matrix = glm::frustum<float>(-right, right, -top, top, near, 20000);

      view_matrix = mat4(1.0f);
      view_matrix = glm::rotate(view_matrix, glm::pi<float>(), vec3(0, 1, 0));
      view_matrix = glm::rotate(view_matrix, cam.pitch, vec3(1, 0, 0));
      view_matrix = glm::rotate(view_matrix, -cam.yaw, vec3(0, 1, 0));
      view_matrix = glm::translate(view_matrix, -cam.pos);
      break;
    }

    case CameraMode::BIRDS_EYE: {
      BirdsEyeCamera &cam = camera.birds_eye_camera;

      float top = 1.0f * cam.span_y / 2.0f;
      float right = top * viewport.size.x / viewport.size.y;
      float y_scale = 1000.0f;
      proj_matrix = glm::transpose(mat4(
            0,          0, 1/right,  0,
        1/top,          0,       0,  0,
            0, -1/y_scale,       0, -1,
            0,          0,       0,  1));

      vec3 pos = camera.rotate_camera.pos;

      view_matrix = mat4(1.0f);
      view_matrix = glm::translate(view_matrix, -cam.pos);
      break;
    }
  }


  glEnable(GL_DEPTH_TEST);
  glDepthFunc(GL_LEQUAL);


  p_surface.program->use();
  p_surface.program->set_uniform("uProjMatrix", proj_matrix);
  p_surface.program->set_uniform("uViewMatrix", view_matrix);
  p_surface.vertex_array->bind();
  p_surface.vertex_array->set("inPos", p_surface.buffers.pos);
  p_surface.vertex_array->set("inColor", p_surface.buffers.color);

  glDrawArrays(GL_TRIANGLES, 0, p_surface.buffers.pos.size());


  p_object.program->use();
  p_object.program->set_uniform("uProjMatrix", proj_matrix);
  p_object.program->set_uniform("uViewMatrix", view_matrix);
  p_object.program->set_uniform("uColor", vec4(1, 0, 0, 1));
  p_object.vertex_array->bind();
  p_object.vertex_array->set("inPos", p_object.buffers.pos);
  glDrawArrays(GL_LINES, 0, p_object.buffers.pos.size());
}
