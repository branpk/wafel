#version 330

uniform mat4 uProjMatrix;
uniform mat4 uViewMatrix;

in vec3 inCenter;
in vec2 inOffset;
in vec3 inColor;

out vec3 vColor;

void main() {
  vec4 center = uProjMatrix * uViewMatrix * vec4(inCenter, 1);
  vec2 screen_offset = 0.01 * inOffset;
  gl_Position = center + center.w * vec4(screen_offset, 0, 0);

  vColor = inColor;
}
