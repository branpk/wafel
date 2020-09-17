#version 450

layout(set = 0, binding = 0) uniform U_Proj {
    mat4 u_Proj;
};
layout(set = 0, binding = 1) uniform U_View {
    mat4 u_View;
};

layout(location = 0) in vec3 a_Center;
layout(location = 1) in vec2 a_Radius;
layout(location = 2) in vec4 a_Color;
layout(location = 3) in vec2 a_Offset;

layout(location = 0) out vec4 v_Color;

void main() {
    vec4 center = u_Proj * u_View * vec4(a_Center, 1);
    vec2 screen_offset = a_Radius * a_Offset;
    gl_Position = center + center.w * vec4(screen_offset, 0, 0);

    v_Color = a_Color;
}
