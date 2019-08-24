#ifndef _GRAPHICS_RENDERER_HPP
#define _GRAPHICS_RENDERER_HPP


#include "util.hpp"
#include "gl_wrappers.hpp"


struct Viewport {
  ivec2 pos;
  ivec2 size;
};


enum class CameraMode {
  ROTATE = 0,
  BIRDS_EYE = 1,
};

struct RotateCamera {
  vec3 pos;
  float pitch;
  float yaw;
  float fov_y;
};

struct BirdsEyeCamera {
  vec3 pos;
  float span_y;
};

struct Camera {
  CameraMode mode;
  union {
    RotateCamera rotate_camera;
    BirdsEyeCamera birds_eye_camera;
  };
};


struct Surface {
  vec3 vertices[3];
  vec3 color;
};

struct Object {
  vec3 pos;
  float hitboxHeight;
  float hitboxRadius;
  // TODO: Move ObjectPath here
};

struct QuarterStep {
  vec3 intended_pos;
  vec3 result_pos;
};

struct ObjectPathNode {
  vec3 pos;
  vector<QuarterStep> quarter_steps;
};

struct ObjectPath {
  vector<ObjectPathNode> nodes;
  size_t root_index;
};


struct Scene {
  Camera camera;
  vector<Surface> surfaces;
  vector<Object> objects;
  vector<ObjectPath> object_paths;
};


struct PathDot {
  vec3 pos;
  vec4 color;
  float radius;
};


class Renderer {
public:
  void render(const Viewport &viewport, const Scene &scene);

private:
  ResourceCache res;
  mat4 proj_matrix, view_matrix;

  void build_transforms(const Viewport &viewport, const Scene &scene);
  void render_surfaces(const Scene &scene);
  void render_objects(const Scene &scene);
  void render_object_paths(const Scene &scene);
  void render_object_path_lines(const Scene &scene);
  void render_path_dots(const vector<PathDot> &dots);
};


#endif
