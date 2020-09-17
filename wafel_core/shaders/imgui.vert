#version 450

layout(set = 0, binding = 0) uniform U_Proj {
    mat4 u_Proj;
};

layout(location = 0) in vec2 a_Pos;
layout(location = 1) in vec2 a_TexCoord;
layout(location = 2) in vec4 a_Color;

layout(location = 0) out vec2 v_TexCoord;
layout(location = 1) out vec4 v_Color;

void main() {
    gl_Position = u_Proj * vec4(a_Pos, 0, 1);
    v_TexCoord = a_TexCoord;
    v_Color = a_Color;
}
