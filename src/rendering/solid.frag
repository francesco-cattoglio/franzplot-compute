// shader.frag
#version 450

layout(location=0) in vec2 v_uv_coords;
layout(location=1) in vec3 v_n_vector;
layout(location=2) flat in int object_id;

layout(location=0) out vec4 f_color;

layout(set = 1, binding = 0) buffer Picking {
    float picking[];
};

//layout(set = 2, binding = 0) uniform texture2D t_diffuse;
//layout(set = 2, binding = 1) uniform sampler s_diffuse;

void main() {
    int pixel_x = int(gl_FragCoord.x);
    int pixel_y = int(gl_FragCoord.y);
    if (pixel_x == 100 && pixel_y == 100) {
        picking[object_id] = gl_FragCoord.z;
    }
//    f_color = texture(sampler2D(t_diffuse, s_diffuse), v_uv_coords);
    f_color.g = 0.0 + 0.8 * v_n_vector.y;
    f_color.b = 0.0 + 0.8 * v_n_vector.z;
    f_color.r = 0.01 + 0.8 * v_n_vector.x; //1.0 - f_color.g - f_color.b;
    f_color.a = 1.0;
}

