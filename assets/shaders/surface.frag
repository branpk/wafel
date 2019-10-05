#version 330

in vec3 vBaryCoords;
in vec4 vColor;

out vec4 outColor;

void main() {
  const float b = 0.8;

  vec3 innerBary = vec3(
    (b+1) * vBaryCoords.x + (b-1) * vBaryCoords.y + (b-1) * vBaryCoords.z,
    (b-1) * vBaryCoords.x + (b+1) * vBaryCoords.y + (b-1) * vBaryCoords.z,
    (b-1) * vBaryCoords.x + (b-1) * vBaryCoords.y + (b+1) * vBaryCoords.z);
  innerBary *= 1/(3*b - 1);

  float t = length(vec3(
    clamp(-innerBary.x, 0, 1),
    clamp(-innerBary.y, 0, 1),
    clamp(-innerBary.z, 0, 1)));
  t *= -(3*b - 1)/(b - 1) * 0.7;

  outColor = vec4(mix(vColor.rgb * 0.5, vColor.rgb * 0.8, t), vColor.a);
}
