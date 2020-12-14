// shader.frag
#version 450

layout(location=0) in vec2 v_uv_coords;
layout(location=1) in vec4 v_n_vector;
layout(location=2) flat in int object_id;
layout(location=3) flat in ivec2 mouse_pos;

layout(location=0) out vec4 f_color;

layout(set = 0, binding = 0) uniform Uniforms {
    mat4 u_view;
    mat4 u_proj;
    ivec2 u_mouse_pos;
    vec2 _u_padding;
};

layout(set = 1, binding = 0) buffer Picking {
    float picking[];
};

layout(set = 2, binding = 0) uniform texture2D t_diffuse;
layout(set = 2, binding = 1) uniform sampler s_diffuse;

void main() {
    highp vec2 muv = vec2(u_view * normalize(v_n_vector))*0.5+vec2(0.5,0.5);
    int pixel_x = int(gl_FragCoord.x);
    int pixel_y = int(gl_FragCoord.y);
    if (pixel_x == mouse_pos.x && pixel_y == mouse_pos.y) {
        picking[object_id] = gl_FragCoord.z;
    }
    f_color = texture(sampler2D(t_diffuse, s_diffuse), vec2(muv.x, 1.0-muv.y));
    //f_color.g = 0.0 + 0.8 * v_n_vector.y;
    //f_color.b = 0.0 + 0.8 * v_n_vector.z;
    //f_color.r = 0.01 + 0.8 * v_n_vector.x; //1.0 - f_color.g - f_color.b;
    f_color.a = 1.0;
}

