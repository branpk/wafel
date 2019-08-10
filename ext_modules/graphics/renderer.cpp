#include <cmath>
#include <algorithm>

#include <glad.h>
#include <glm/glm.hpp>

#include "renderer.hpp"

using namespace std;

Renderer::Renderer(int screen_width, int screen_height)
  : screen_size(max(screen_width, 1), max(screen_height, 1))
{
  glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
}

void Renderer::set_camera(vec3 pos, float pitch, float yaw, float fov_y) {
  float near_v = 10;
  float top = near_v * tanf(fov_y / 2);
  float right = top * screen_size.x / screen_size.y;

  glMatrixMode(GL_PROJECTION);
  glLoadIdentity();
  glFrustum(-right, right, -top, top, near_v, 20000);

  glMatrixMode(GL_MODELVIEW);
  glLoadIdentity();
  glRotatef(180, 0, 1, 0);
  glRotatef(pitch * 180 / 3.14159f, 1, 0, 0);
  glRotatef(-yaw * 180 / 3.14159f, 0, 1, 0);
  glTranslatef(-pos.x, -pos.y, -pos.z);
}

void Renderer::add_surface(vec3 v1, vec3 v2, vec3 v3) {
  glColor3f(0.7, 0.7, 0.7);
  glBegin(GL_TRIANGLES);
  glVertex3f(v1.x, v1.y, v1.z);
  glVertex3f(v2.x, v2.y, v2.z);
  glVertex3f(v3.x, v3.y, v3.z);
  glEnd();

  glColor3f(0, 0, 0);
  glBegin(GL_LINE_LOOP);
  glVertex3f(v1.x, v1.y, v1.z);
  glVertex3f(v2.x, v2.y, v2.z);
  glVertex3f(v3.x, v3.y, v3.z);
  glEnd();
}

void Renderer::add_object(vec3 pos, float height) {
  glColor3f(1, 0, 0);
  glBegin(GL_LINES);
  glVertex3f(pos.x, pos.y, pos.z);
  glVertex3f(pos.x, pos.y + height, pos.z);
  glEnd();
}

void Renderer::render() {
}
