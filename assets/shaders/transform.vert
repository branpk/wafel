#version 330

uniform mat4 uProjMatrix;
uniform mat4 uViewMatrix;

in vec3 inPos;

void main() {
  gl_Position = uProjMatrix * uViewMatrix * vec4(inPos, 1);
}
