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
};

struct ObjectPath {
  vector<vec3> pos;
};


struct Scene {
  Camera camera;
  vector<Surface> surfaces;
  vector<Object> objects;
  vector<ObjectPath> object_paths;
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
};


#endif
