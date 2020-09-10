#version 450

layout(location=0) in vec3 a_position;
layout(location=1) in vec2 uv_coords;
layout(location=2) in vec3 n_vectors;

layout(location=0) out vec2 v_uv_coords;

layout(set = 1, binding = 0) uniform Uniforms {
    mat4 u_view_proj;
};
layout(set = 1, binding = 1) buffer ValuesFromCompute {
    float v_coords[];
};

void main() {
    v_uv_coords = uv_coords;
    vec4 position;
    position.xyz = a_position;
    position.z = v_coords[gl_VertexIndex*4+2];
    position.y = v_coords[gl_VertexIndex*4+1];
    position.w = 1.0;
    //v_coords[gl_VertexIndex] = a_position.x;
    //position.x = v_coords[gl_VertexIndex];
    //position.y = v_coords[gl_VertexIndex+1];
    //position.z = v_coords[gl_VertexIndex+2];
    //position.w = v_coords[gl_VertexIndex+3];
    gl_Position = u_view_proj * position;
    //gl_Position = position;
}

