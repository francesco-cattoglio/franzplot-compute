#version 450

layout(location=0) in vec4 a_position;
layout(location=1) in vec4 n_vectors;
layout(location=2) in vec2 uv_coords;
layout(location=3) in vec2 _v_padding;

layout(location=0) out vec2 v_uv_coords;
layout(location=1) out vec4 v_n_vector;
layout(location=2) out int object_idx;
layout(location=3) out ivec2 mouse_pos;

layout(set = 0, binding = 0) uniform Uniforms {
    mat4 u_view;
    mat4 u_proj;
    ivec2 u_mouse_pos;
    int highlight_id;
    float _u_padding;
};

void main() {
    v_uv_coords = uv_coords.xy;
    mouse_pos = u_mouse_pos;
    object_idx = gl_InstanceIndex;
    v_n_vector = n_vectors;
    gl_Position = u_proj * u_view * a_position;
}

