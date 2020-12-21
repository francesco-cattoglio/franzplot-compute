#version 450

layout(location=0) in vec3 in_position;
layout(location=1) in vec4 in_color;

layout(location=0) out vec4 color;

layout(set = 0, binding = 0) uniform Uniforms {
    mat4 u_view;
    mat4 u_proj;
    ivec2 u_mouse_pos;
    vec2 _u_padding;
};

void main() {
    color = in_color;
    gl_Position = u_proj * u_view * vec4(in_position, 1.0);
}

