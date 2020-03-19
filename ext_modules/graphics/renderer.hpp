#ifndef _GRAPHICS_RENDERER_HPP
#define _GRAPHICS_RENDERER_HPP


#include "util.hpp"
#include "gl_wrappers.hpp"
#include "scene.hpp"


struct PathDot {
  vec3 pos;
  vec4 color;
  float radius;
};


class Renderer {
public:
  Renderer(const string &assets_directory);

  void render(const Scene &scene);

private:
  string assets_directory;
  ResourceCache res;
  mat4 proj_matrix, view_matrix;

  void build_transforms(const Scene &scene);
  void render_surfaces(const Scene &scene, bool hidden);
  void render_wall_hitboxes(const Scene &scene);
  void render_wall_hitbox_tris(const Scene &scene);
  void render_wall_hitbox_lines(const Scene &scene);
  void render_objects(const Scene &scene);
  void render_object_paths(const Scene &scene);
  void render_object_path_lines(const Scene &scene);
  void render_path_dots(const Scene &scene, const vector<PathDot> &dots);
  void render_camera_target(const Scene &scene);
  void render_unit_squares(const Scene &scene);
};


#endif
