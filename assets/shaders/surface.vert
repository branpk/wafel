#version 330

uniform mat4 uProjMatrix;
uniform mat4 uViewMatrix;

in vec3 inPos;
in vec4 inColor;

out vec3 vBaryCoords;
out vec4 vColor;

void main() {
  gl_Position = uProjMatrix * uViewMatrix * vec4(inPos, 1);

  vec3[3] baryCoords = vec3[3](
    vec3(1, 0, 0),
    vec3(0, 1, 0),
    vec3(0, 0, 1));
  vBaryCoords = baryCoords[gl_VertexID % 3];

  vColor = inColor;
}
