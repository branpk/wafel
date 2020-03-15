#ifndef _GRAPHICS_SCENE_HPP
#define _GRAPHICS_SCENE_HPP


#include "util.hpp"


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


enum class SurfaceType {
  FLOOR,
  CEILING,
  WALL_X_PROJ,
  WALL_Z_PROJ,
};

struct Surface {
  SurfaceType type;
  array<vec3, 3> vertices;
  vec3 normal;
};

struct Object {
  vec3 pos;
  float hitbox_height;
  float hitbox_radius;
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


#endif
