#version 450

layout(location = 0) in vec3 v_Bary;
layout(location = 1) in vec4 v_Color;

layout(location = 0) out vec4 o_Color;

void main() {
    const float b = 0.8;

    vec3 inner_bary = vec3(
        (b+1) * v_Bary.x + (b-1) * v_Bary.y + (b-1) * v_Bary.z,
        (b-1) * v_Bary.x + (b+1) * v_Bary.y + (b-1) * v_Bary.z,
        (b-1) * v_Bary.x + (b-1) * v_Bary.y + (b+1) * v_Bary.z);
    inner_bary *= 1/(3*b - 1);

    float t = length(vec3(
        clamp(-inner_bary.x, 0, 1),
        clamp(-inner_bary.y, 0, 1),
        clamp(-inner_bary.z, 0, 1)));
    t *= -(3*b - 1)/(b - 1) * 0.7;

    vec4 inner_color = vec4(v_Color.rgb * 0.5, v_Color.a);
    vec4 outer_color = vec4(v_Color.rgb * 0.8, max(v_Color.a, 0.8));
    o_Color = mix(inner_color, outer_color, t);
}
