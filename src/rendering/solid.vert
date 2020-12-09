#version 450

layout(location=0) in vec4 a_position;
layout(location=1) in vec4 n_vectors;
layout(location=2) in vec4 uv_coords;

layout(location=0) out vec2 v_uv_coords;
layout(location=1) out vec3 v_n_vector;
layout(location=2) out int object_id;
layout(location=3) out vec2 mouse_pos;

layout(set = 0, binding = 0) uniform Uniforms {
    mat4 u_view_proj;
    vec2 u_mouse_pos;
    vec2 _padding;
};

void main() {
    v_uv_coords = uv_coords.xy;
    mouse_pos = u_mouse_pos;
    object_id = gl_InstanceIndex;
    v_n_vector = n_vectors.xyz;
    v_n_vector.x = gl_InstanceIndex * 0.25;
    gl_Position = u_view_proj * a_position;
}

