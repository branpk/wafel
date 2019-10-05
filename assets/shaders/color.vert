#version 330

uniform mat4 uProjMatrix;
uniform mat4 uViewMatrix;

in vec3 inPos;
in vec4 inColor;

out vec4 vColor;

void main() {
  gl_Position = uProjMatrix * uViewMatrix * vec4(inPos, 1);
  vColor = inColor;
}
