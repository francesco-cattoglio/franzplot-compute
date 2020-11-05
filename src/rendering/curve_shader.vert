#version 450

layout(location=0) in vec4 a_position;

layout(location=0) out vec2 v_uv_coords;
layout(location=1) out vec4 v_n_vector;

layout(set = 0, binding = 0) buffer CurveData {
    vec4 curvedata[];
};
layout(set = 1, binding = 0) uniform Uniforms {
    mat4 u_view_proj;
};

void main() {
    v_uv_coords = vec2(0.5, 0.5);
    int circle_id = gl_VertexIndex/3;
    vec4 circle_position  = curvedata[circle_id*3];
    vec4 circle_forward   = curvedata[circle_id*3+1];
    vec4 circle_up        = curvedata[circle_id*3+2];
    vec3 circle_left      = -1.0 * normalize(cross(circle_forward.xyz, circle_up.xyz));
    mat4 new_basis;
    new_basis[0] = circle_forward;
    new_basis[1] = vec4(circle_left, 0.0);
    new_basis[2] = circle_up;
    new_basis[3] = vec4(0.0, 0.0, 0.0, 1.0);
    mat4 circle_transform = new_basis;
    vec4 vertex_position = circle_transform * a_position + circle_position;

    vec3 circle_normal = normalize(a_position.xyz);
    v_n_vector = vec4(circle_normal, 0.0);
    gl_Position = u_view_proj * vertex_position;
}

