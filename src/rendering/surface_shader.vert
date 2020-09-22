#version 450

layout(location=0) in vec3 a_position;
layout(location=1) in vec2 uv_coords;
layout(location=2) in vec3 n_vectors;

layout(location=0) out vec2 v_uv_coords;

layout(set = 1, binding = 0) uniform Uniforms {
    mat4 u_view_proj;
};

void main() {
    v_uv_coords = uv_coords;
    vec4 position;
    position.xyz = a_position;
    position.w = 1.0;
    gl_Position = u_view_proj * position;
}

