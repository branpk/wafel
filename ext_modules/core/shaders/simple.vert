#version 450

layout(set = 0, binding = 0) uniform U_Proj {
    mat4 u_Proj;
};
layout(set = 0, binding = 1) uniform U_View {
    mat4 u_View;
};

layout(location = 0) in vec3 a_Pos;

void main() {
    gl_Position = u_Proj * u_View * vec4(a_Pos, 1);
}
