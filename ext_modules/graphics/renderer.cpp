#include <cmath>
#include <algorithm>
#include <cstdio>

#include <glad.h>
#include <glm/glm.hpp>
#include <glm/gtc/matrix_transform.hpp>

#include "renderer.hpp"

using namespace std;


void Renderer::render(const Viewport &viewport, const Scene &scene) {
  glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

  glViewport(viewport.pos.x, viewport.pos.y, viewport.size.x, viewport.size.y);

  glEnable(GL_CULL_FACE);
  glCullFace(GL_BACK);
  glFrontFace(GL_CCW);

  glEnable(GL_DEPTH_TEST);
  glDepthFunc(GL_LEQUAL);

  glEnable(GL_BLEND);
  glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);

  build_transforms(viewport, scene);
  render_surfaces(scene);
  render_objects(scene);
  render_object_paths(scene);
}

void Renderer::build_transforms(const Viewport &viewport, const Scene &scene) {
  switch (scene.camera.mode) {
    case CameraMode::ROTATE: {
      const RotateCamera &camera = scene.camera.rotate_camera;

      float near = 10;
      float top = near * tanf(camera.fov_y / 2);
      float right = top * viewport.size.x / viewport.size.y;
      proj_matrix = glm::frustum<float>(-right, right, -top, top, near, 20000);

      view_matrix = mat4(1.0f);
      view_matrix = glm::rotate(view_matrix, glm::pi<float>(), vec3(0, 1, 0));
      view_matrix = glm::rotate(view_matrix, camera.pitch, vec3(1, 0, 0));
      view_matrix = glm::rotate(view_matrix, -camera.yaw, vec3(0, 1, 0));
      view_matrix = glm::translate(view_matrix, -camera.pos);
      break;
    }

    case CameraMode::BIRDS_EYE: {
      const BirdsEyeCamera &camera = scene.camera.birds_eye_camera;

      float top = 1.0f * camera.span_y / 2.0f;
      float right = top * viewport.size.x / viewport.size.y;
      float y_scale = 1000.0f;
      proj_matrix = glm::transpose(mat4(
            0,          0, 1/right,  0,
        1/top,          0,       0,  0,
            0, -1/y_scale,       0, -1,
            0,          0,       0,  1));

      vec3 pos = camera.pos;

      view_matrix = mat4(1.0f);
      view_matrix = glm::translate(view_matrix, -camera.pos);
      break;
    }
  }
}

void Renderer::render_surfaces(const Scene &scene) {
  Program *program = res.program(
    "assets/shaders/surface.vert",
    "assets/shaders/surface.frag");

  program->use();
  program->set_uniform("uProjMatrix", proj_matrix);
  program->set_uniform("uViewMatrix", view_matrix);

  vector<vec3> in_pos;
  vector<vec3> in_color;

  for (const Surface &surface : scene.surfaces) {
    in_pos.push_back(surface.vertices[0]);
    in_pos.push_back(surface.vertices[1]);
    in_pos.push_back(surface.vertices[2]);

    in_color.insert(in_color.end(), 3, surface.color);
  }

  VertexArray *vertex_array = new VertexArray(program);
  vertex_array->bind();
  vertex_array->set("inPos", in_pos);
  vertex_array->set("inColor", in_color);

  glDrawArrays(GL_TRIANGLES, 0, in_pos.size());
  delete vertex_array;
}

void Renderer::render_objects(const Scene &scene) {
  Program *program = res.program(
    "assets/shaders/color.vert",
    "assets/shaders/color.frag");

  program->use();
  program->set_uniform("uProjMatrix", proj_matrix);
  program->set_uniform("uViewMatrix", view_matrix);

  vector<vec3> in_pos;
  vector<vec4> in_color;

  for (const Object &object : scene.objects) {
    in_pos.push_back(object.pos);
    in_pos.push_back(object.pos + vec3(0, object.hitboxHeight, 0));
    in_color.insert(in_color.end(), 2, vec4(1, 0, 0, 1));

    if (object.hitboxRadius > 0) {
      const int num_edges = 64;
      for (int i = 0; i < num_edges; i++) {
        float a0 = (float)i / (float)num_edges * 2 * glm::pi<float>();
        float a1 = (float)(i + 1) / (float)num_edges * 2 * glm::pi<float>();

        vec3 offset0 = object.hitboxRadius * vec3(sinf(a0), 0, cosf(a0));
        vec3 offset1 = object.hitboxRadius * vec3(sinf(a1), 0, cosf(a1));

        in_pos.push_back(object.pos + offset0);
        in_pos.push_back(object.pos + offset1);
      }
      in_color.insert(in_color.end(), 2 * num_edges, vec4(1, 0, 0, 1));
    }
  }

  VertexArray *vertex_array = new VertexArray(program);
  vertex_array->bind();
  vertex_array->set("inPos", in_pos);
  vertex_array->set("inColor", in_color);

  glDrawArrays(GL_LINES, 0, in_pos.size());
  delete vertex_array;
}

float get_path_alpha(const ObjectPath &path, size_t index) {
  int rel_index = (int)index - (int)path.root_index;

  float t;
  if (rel_index > 0) {
    t = (float)rel_index / (float)(path.pos.size() - path.root_index - 1);
  } else if (rel_index < 0) {
    t = -(float)rel_index / (float)path.root_index;
  } else {
    t = 0;
  }

  return 1 - t;
}

void Renderer::render_object_paths(const Scene &scene) {
  render_object_path_lines(scene);

  vector<PathDot> path_dots;
  for (const ObjectPath &path : scene.object_paths) {
    size_t index = 0;
    for (vec3 pos : path.pos) {
      path_dots.push_back({
        pos,
        vec4(1, 0, 0, get_path_alpha(path, index)),
      });
      index += 1;
    }
  }
  render_path_dots(path_dots);
}

void Renderer::render_object_path_lines(const Scene &scene) {
  Program *program = res.program(
    "assets/shaders/color.vert",
    "assets/shaders/color.frag");

  program->use();
  program->set_uniform("uProjMatrix", proj_matrix);
  program->set_uniform("uViewMatrix", view_matrix);

  VertexArray *vertex_array = new VertexArray(program);
  vertex_array->bind();

  for (const ObjectPath &path : scene.object_paths) {
    vector<vec3> in_pos;
    vector<vec4> in_color;

    size_t index = 0;
    for (vec3 it : path.pos) {
      in_pos.push_back(it + vec3(0, 0.01f, 0));
      in_color.push_back(vec4(0.5f, 0, 0, get_path_alpha(path, index)));
      index += 1;
    }

    vertex_array->set("inPos", in_pos);
    vertex_array->set("inColor", in_color);
    glDrawArrays(GL_LINE_STRIP, 0, in_pos.size());
  }

  delete vertex_array;
}

void Renderer::render_path_dots(const vector<PathDot> &dots) {
  // TODO: Could do triangle fans with indexing

  Program *program = res.program(
    "assets/shaders/path_dot.vert",
    "assets/shaders/color.frag");

  program->use();
  program->set_uniform("uProjMatrix", proj_matrix);
  program->set_uniform("uViewMatrix", view_matrix);

  vector<vec3> in_center;
  vector<vec2> in_offset;
  vector<vec4> in_color;

  for (const PathDot &dot : dots) {
    const int num_edges = 12;

    in_center.insert(in_center.end(), 3 * num_edges, dot.pos + vec3(0, 0.01f, 0));
    in_color.insert(in_color.end(), 3 * num_edges, dot.color);

    for (int i = 0; i < num_edges; i++) {
      float a0 = (float)i / (float)num_edges * 2 * glm::pi<float>();
      float a1 = (float)(i + 1) / (float)num_edges * 2 * glm::pi<float>();

      in_offset.push_back(vec2(0, 0));
      in_offset.push_back(vec2(cosf(a0), sinf(a0)));
      in_offset.push_back(vec2(cosf(a1), sinf(a1)));
    }
  }

  VertexArray *vertex_array = new VertexArray(program);
  vertex_array->bind();
  vertex_array->set("inCenter", in_center);
  vertex_array->set("inOffset", in_offset);
  vertex_array->set("inColor", in_color);

  glDrawArrays(GL_TRIANGLES, 0, in_center.size());
  delete vertex_array;
}
