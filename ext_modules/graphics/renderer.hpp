#ifndef _GRAPHICS_RENDERER_H
#define _GRAPHICS_RENDERER_H

#include <glm/vec2.hpp>
#include <glm/vec3.hpp>

using glm::ivec2;
using glm::vec2;
using glm::vec3;


class Renderer {
public:
  Renderer(int screen_width, int screen_height);
  void set_camera(vec3 pos, float pitch, float yaw, float fov_y);
  void add_surface(vec3 v1, vec3 v2, vec3 v3);
  void add_object(vec3 pos, float height);
  void render();

private:
  ivec2 screen_size;
};


#endif
