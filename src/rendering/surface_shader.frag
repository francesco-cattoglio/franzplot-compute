// shader.frag
#version 450

layout(location=0) in vec2 v_uv_coords;
layout(location=1) in vec3 v_n_vector;
layout(location=0) out vec4 f_color;

layout(set = 1, binding = 0) uniform texture2D t_diffuse;
layout(set = 1, binding = 1) uniform sampler s_diffuse;

void main() {
    f_color = texture(sampler2D(t_diffuse, s_diffuse), v_uv_coords);
    f_color.r = v_n_vector.x*v_n_vector.x;
    f_color.g = v_n_vector.y;
    f_color.b = v_n_vector.z;
}

