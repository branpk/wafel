#include <cmath>
#include <algorithm>
#include <cstdio>

#include <glad.h>
#include <glm/glm.hpp>
#include <glm/gtc/matrix_transform.hpp>

#include "renderer.hpp"

using namespace std;


Renderer::Renderer(const string &assets_directory)
  : assets_directory(assets_directory)
{}


void Renderer::render(const Scene &scene) {
  const Viewport &viewport = scene.viewport;

  glEnable(GL_SCISSOR_TEST);
  glScissor(viewport.pos.x, viewport.pos.y, viewport.size.x, viewport.size.y);
  glViewport(viewport.pos.x, viewport.pos.y, viewport.size.x, viewport.size.y);

  glDepthMask(GL_TRUE);

  glClearColor(0.0f, 0.0f, 0.0f, 1.0f);
  glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

  // glEnable(GL_CULL_FACE);
  // glCullFace(GL_BACK);
  // glFrontFace(GL_CCW);

  glEnable(GL_DEPTH_TEST);
  glDepthFunc(GL_LEQUAL);

  glEnable(GL_BLEND);
  glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);

  build_transforms(scene);
  render_surfaces(scene);
  render_objects(scene);
  render_object_paths(scene);
  render_wall_hitboxes(scene);
  if (scene.camera.mode == CameraMode::ROTATE) {
    render_camera_target(scene);
  }
  if (scene.camera.mode == CameraMode::BIRDS_EYE) {
    render_unit_squares(scene);
  }
}

void Renderer::build_transforms(const Scene &scene) {
  const Viewport &viewport = scene.viewport;

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
      float y_scale = 20000.0f;
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
    assets_directory + "/shaders/surface.vert",
    assets_directory + "/shaders/surface.frag");

  program->use();
  program->set_uniform("uProjMatrix", proj_matrix);
  program->set_uniform("uViewMatrix", view_matrix);

  vector<vec3> in_pos;
  vector<vec4> in_color;

  for (const Surface &surface : scene.surfaces) {
    in_pos.push_back(surface.vertices[0]);
    in_pos.push_back(surface.vertices[1]);
    in_pos.push_back(surface.vertices[2]);

    vec3 color;
    switch (surface.type) {
    case SurfaceType::FLOOR: color = vec3(0.5f, 0.5f, 1.0f); break;
    case SurfaceType::CEILING: color = vec3(1.0f, 0.5f, 0.5f); break;
    case SurfaceType::WALL_X_PROJ: color = vec3(0.3f, 0.8f, 0.3f); break;
    case SurfaceType::WALL_Z_PROJ: color = vec3(0.15f, 0.4f, 0.15f); break;
    }
    in_color.insert(in_color.end(), 3, vec4(color, 1));
  }

  VertexArray *vertex_array = new VertexArray(program);
  vertex_array->bind();
  vertex_array->set("inPos", in_pos);
  vertex_array->set("inColor", in_color);

  glDrawArrays(GL_TRIANGLES, 0, in_pos.size());
  delete vertex_array;
}

void Renderer::render_wall_hitboxes(const Scene &scene) {
  render_wall_hitbox_tris(scene);
  render_wall_hitbox_lines(scene);
}

void Renderer::render_wall_hitbox_tris(const Scene &scene) {
  Program *program = res.program(
    assets_directory + "/shaders/color.vert",
    assets_directory + "/shaders/color.frag");

  program->use();
  program->set_uniform("uProjMatrix", proj_matrix);
  program->set_uniform("uViewMatrix", view_matrix);

  vector<vec3> in_pos;
  vector<vec4> in_color;

  for (const Surface &surface : scene.surfaces) {
    if (surface.type == SurfaceType::WALL_X_PROJ ||
      surface.type == SurfaceType::WALL_Z_PROJ)
    {
      vec3 proj_dir =
        surface.type == SurfaceType::WALL_X_PROJ
          ? vec3(1, 0, 0)
          : vec3(0, 0, 1);
      float proj_dist = 50.0f / glm::dot(surface.normal, proj_dir);

      vec3 ext_vertices[3] = {
        surface.vertices[0] + proj_dist * proj_dir,
        surface.vertices[1] + proj_dist * proj_dir,
        surface.vertices[2] + proj_dist * proj_dir,
      };

      vec3 int_vertices[3] = {
        surface.vertices[0] - proj_dist * proj_dir,
        surface.vertices[1] - proj_dist * proj_dir,
        surface.vertices[2] - proj_dist * proj_dir,
      };

      in_pos.push_back(ext_vertices[0]);
      in_pos.push_back(ext_vertices[1]);
      in_pos.push_back(ext_vertices[2]);

      for (int i0 = 0; i0 < 3; i0++) {
        int i1 = (i0 + 1) % 3;
        in_pos.push_back(int_vertices[i0]);
        in_pos.push_back(int_vertices[i1]);
        in_pos.push_back(ext_vertices[i0]);
        in_pos.push_back(ext_vertices[i0]);
        in_pos.push_back(int_vertices[i1]);
        in_pos.push_back(ext_vertices[i1]);
      }

      vec4 color =
        surface.type == SurfaceType::WALL_X_PROJ
          ? vec4(0.3f, 0.8f, 0.3f, 0.4f)
          : vec4(0.15f, 0.4f, 0.15f, 0.4f);
      in_color.insert(in_color.end(), in_pos.size() - in_color.size(), color);
    }
  }

  VertexArray *vertex_array = new VertexArray(program);
  vertex_array->bind();
  vertex_array->set("inPos", in_pos);
  vertex_array->set("inColor", in_color);

  glDepthMask(GL_FALSE);
  glDrawArrays(GL_TRIANGLES, 0, in_pos.size());
  glDepthMask(GL_TRUE);
  delete vertex_array;
}

void Renderer::render_wall_hitbox_lines(const Scene &scene) {
  Program *program = res.program(
    assets_directory + "/shaders/color.vert",
    assets_directory + "/shaders/color.frag");

  program->use();
  program->set_uniform("uProjMatrix", proj_matrix);
  program->set_uniform("uViewMatrix", view_matrix);

  vector<vec3> in_pos;
  vector<vec4> in_color;

  for (const Surface &surface : scene.surfaces) {
    if (surface.type == SurfaceType::WALL_X_PROJ ||
      surface.type == SurfaceType::WALL_Z_PROJ)
    {
      vec3 proj_dir =
        surface.type == SurfaceType::WALL_X_PROJ
          ? vec3(1, 0, 0)
          : vec3(0, 0, 1);
      float proj_dist = 50.0f / glm::dot(surface.normal, proj_dir);

      vec3 ext_vertices[3] = {
        surface.vertices[0] + proj_dist * proj_dir,
        surface.vertices[1] + proj_dist * proj_dir,
        surface.vertices[2] + proj_dist * proj_dir,
      };

      vec3 int_vertices[3] = {
        surface.vertices[0] - proj_dist * proj_dir,
        surface.vertices[1] - proj_dist * proj_dir,
        surface.vertices[2] - proj_dist * proj_dir,
      };

      for (int i0 = 0; i0 < 3; i0++) {
        int i1 = (i0 + 1) % 3;
        in_pos.push_back(int_vertices[i0]);
        in_pos.push_back(ext_vertices[i0]);
        in_pos.push_back(int_vertices[i0]);
        in_pos.push_back(int_vertices[i1]);
        in_pos.push_back(ext_vertices[i0]);
        in_pos.push_back(ext_vertices[i1]);
      }

      in_color.insert(in_color.end(), in_pos.size() - in_color.size(), vec4(0, 0, 0, 0.5f));
    }
  }

  VertexArray *vertex_array = new VertexArray(program);
  vertex_array->bind();
  vertex_array->set("inPos", in_pos);
  vertex_array->set("inColor", in_color);

  glDrawArrays(GL_LINES, 0, in_pos.size());
  delete vertex_array;
}

void Renderer::render_objects(const Scene &scene) {
  Program *program = res.program(
    assets_directory + "/shaders/color.vert",
    assets_directory + "/shaders/color.frag");

  program->use();
  program->set_uniform("uProjMatrix", proj_matrix);
  program->set_uniform("uViewMatrix", view_matrix);

  vector<vec3> in_pos;
  vector<vec4> in_color;

  for (const Object &object : scene.objects) {
    in_pos.push_back(object.pos);
    in_pos.push_back(object.pos + vec3(0, object.hitbox_height, 0));
    in_color.insert(in_color.end(), 2, vec4(1, 0, 0, 1));

    if (object.hitbox_radius > 0) {
      const int num_edges = 64;
      for (int i = 0; i < num_edges; i++) {
        float a0 = (float) i / (float) num_edges * 2 * glm::pi<float>();
        float a1 = (float) (i + 1) / (float) num_edges * 2 * glm::pi<float>();

        vec3 offset0 = object.hitbox_radius * vec3(sinf(a0), 0, cosf(a0));
        vec3 offset1 = object.hitbox_radius * vec3(sinf(a1), 0, cosf(a1));

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
  int rel_index = (int) index - (int) path.root_index;

  float t;
  if (rel_index > 0) {
    t = (float) rel_index / (float) (path.nodes.size() - path.root_index - 1);
  } else if (rel_index < 0) {
    t = -(float) rel_index / (float) path.root_index;
  } else {
    t = 0;
  }

  return 1 - t;
}

void Renderer::render_object_paths(const Scene &scene) {
  render_object_path_lines(scene);

  vector<PathDot> path_dots;
  for (const ObjectPath &path : scene.object_paths) {
    for (size_t i = 0; i < path.nodes.size(); i++) {
      const ObjectPathNode &node = path.nodes[i];

      float alpha = get_path_alpha(path, i);
      path_dots.push_back({
        node.pos,
        vec4(1, 0, 0, alpha),
        0.01f,
      });

      for (const QuarterStep &qstep : node.quarter_steps) {
        if (qstep.intended_pos != qstep.result_pos) {
          path_dots.push_back({
            qstep.intended_pos,
            vec4(0.8f, 0.5f, 0.8f, alpha),
            0.008f,
          });
        }

        if (i == path.nodes.size() - 1 || qstep.result_pos != path.nodes[i + 1].pos) {
          path_dots.push_back({
            qstep.result_pos,
            vec4(1, 0.5f, 0.0f, alpha),
            0.008f,
          });
        }
      }
    }
  }

  render_path_dots(scene, path_dots);
}

void Renderer::render_object_path_lines(const Scene &scene) {
  Program *program = res.program(
    assets_directory + "/shaders/color.vert",
    assets_directory + "/shaders/color.frag");

  program->use();
  program->set_uniform("uProjMatrix", proj_matrix);
  program->set_uniform("uViewMatrix", view_matrix);

  VertexArray *vertex_array = new VertexArray(program);
  vertex_array->bind();

  for (const ObjectPath &path : scene.object_paths) {
    vector<vec3> in_pos;
    vector<vec4> in_color;

    size_t index = 0;
    for (const ObjectPathNode &node : path.nodes) {
      vec4 color = vec4(0.5f, 0, 0, get_path_alpha(path, index));

      in_pos.push_back(node.pos + vec3(0, 0.01f, 0));
      in_color.push_back(color);

      for (const QuarterStep &qstep : node.quarter_steps) {
        in_pos.push_back(qstep.intended_pos + vec3(0, 0.01f, 0));
        in_pos.push_back(qstep.result_pos + vec3(0, 0.01f, 0));
        in_color.insert(in_color.end(), 2, color);
      }

      index += 1;
    }

    vertex_array->set("inPos", in_pos);
    vertex_array->set("inColor", in_color);
    glDrawArrays(GL_LINE_STRIP, 0, in_pos.size());
  }

  delete vertex_array;
}

void Renderer::render_path_dots(const Scene &scene, const vector<PathDot> &dots) {
  // TODO: Could do triangle fans with indexing

  Program *program = res.program(
    assets_directory + "/shaders/path_dot.vert",
    assets_directory + "/shaders/color.frag");

  program->use();
  program->set_uniform("uProjMatrix", proj_matrix);
  program->set_uniform("uViewMatrix", view_matrix);

  vector<vec3> in_center;
  vector<vec2> in_offset;
  vector<vec4> in_color;
  vector<vec2> in_radius;

  for (const PathDot &dot : dots) {
    const int num_edges = 12;

    in_center.insert(in_center.end(), 3 * num_edges, dot.pos + vec3(0, 0.01f, 0));
    in_color.insert(in_color.end(), 3 * num_edges, dot.color);
    float x_radius = dot.radius * scene.viewport.size.y / scene.viewport.size.x;
    in_radius.insert(in_radius.end(), 3 * num_edges, vec2(x_radius, dot.radius));

    for (int i = 0; i < num_edges; i++) {
      float a0 = (float) i / (float) num_edges * 2 * glm::pi<float>();
      float a1 = (float) (i + 1) / (float) num_edges * 2 * glm::pi<float>();

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
  vertex_array->set("inRadius", in_radius);

  glDrawArrays(GL_TRIANGLES, 0, in_center.size());
  delete vertex_array;
}

void Renderer::render_camera_target(const Scene &scene) {
  RotateCamera camera = scene.camera.rotate_camera;
  if (!camera.has_target) {
    return;
  }

  vector<PathDot> dots;
  dots.push_back({
    camera.target,
    vec4(0.2, 0.2, 0.2, 0.8),
    0.01f,
  });

  render_path_dots(scene, dots);

  Program *program = res.program(
    assets_directory + "/shaders/color.vert",
    assets_directory + "/shaders/color.frag");

  program->use();
  program->set_uniform("uProjMatrix", proj_matrix);
  program->set_uniform("uViewMatrix", view_matrix);

  vector<vec3> in_pos;
  in_pos.push_back(camera.target);
  in_pos.push_back(camera.target + vec3(0, -10000, 0));

  vector<vec4> in_color;
  in_color.insert(in_color.end(), 2, vec4(0.2, 0.2, 0.2, 0.8));

  VertexArray *vertex_array = new VertexArray(program);
  vertex_array->bind();
  vertex_array->set("inPos", in_pos);
  vertex_array->set("inColor", in_color);

  glDrawArrays(GL_LINES, 0, in_pos.size());

  delete vertex_array;
}

void Renderer::render_unit_squares(const Scene &scene) {
  Program *program = res.program(
    assets_directory + "/shaders/color.vert",
    assets_directory + "/shaders/color.frag");

  program->use();
  program->set_uniform("uProjMatrix", proj_matrix);
  program->set_uniform("uViewMatrix", view_matrix);

  BirdsEyeCamera camera = scene.camera.birds_eye_camera;

  float span_x = camera.span_y;
  float span_z = span_x * scene.viewport.size.x / scene.viewport.size.y;

  float min_x = camera.pos.x - span_x / 2.0f;
  float max_x = camera.pos.x + span_x / 2.0f;
  float min_z = camera.pos.z - span_z / 2.0f;
  float max_z = camera.pos.z + span_z / 2.0f;

  float density_threshold = 0.1f;
  float density = fmax(
    (max_x - min_x) / scene.viewport.size.y,
    (max_z - min_z) / scene.viewport.size.x);
  if (density > density_threshold) {
    return;
  }

  VertexArray *vertex_array = new VertexArray(program);
  vertex_array->bind();

  vector<vec3> in_pos;

  for (int x = (int) min_x; x <= (int) max_x; x++) {
    in_pos.push_back(vec3(x, camera.pos.y - 1, min_z));
    in_pos.push_back(vec3(x, camera.pos.y - 1, max_z));
  }
  for (int z = (int) min_z; z <= (int) max_z; z++) {
    in_pos.push_back(vec3(min_x, camera.pos.y - 1, z));
    in_pos.push_back(vec3(max_x, camera.pos.y - 1, z));
  }

  vector<vec4> in_color;
  in_color.insert(in_color.end(), in_pos.size(), vec4(0.8f, 0.8f, 1.0f, 0.5f));

  vertex_array->set("inPos", in_pos);
  vertex_array->set("inColor", in_color);

  glDisable(GL_DEPTH_TEST);
  glDrawArrays(GL_LINES, 0, in_pos.size());
  glEnable(GL_DEPTH_TEST);

  delete vertex_array;
}
