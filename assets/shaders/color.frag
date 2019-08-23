#version 330

in vec3 vColor;

out vec4 outColor;

void main() {
  outColor = vec4(vColor, 1);
}
