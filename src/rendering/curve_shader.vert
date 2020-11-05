#version 450

layout(location=0) in vec4 a_position;

layout(location=0) out vec2 v_uv_coords;
layout(location=1) out vec3 v_n_vector;

layout(set = 0, binding = 0) buffer CurveData {
    vec4 curvedata[];
};
layout(set = 1, binding = 0) uniform Uniforms {
    mat4 u_view_proj;
};

void main() {
    v_uv_coords = vec2(0.5, 0.5);
    int circle_id = gl_VertexIndex/3;
    vec4 vertex_position = a_position + curvedata[circle_id*3];
    v_n_vector = normalize(vertex_position.xyz);
    gl_Position = u_view_proj * vertex_position;
}

