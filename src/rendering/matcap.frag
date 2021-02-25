#version 450

layout(location=0) in vec2 v_uv_coords;
layout(location=1) in vec4 v_n_vector;
layout(location=2) flat in int object_idx;

layout(location=0) out vec4 f_color;

layout(set = 0, binding = 0) uniform Uniforms {
    mat4 u_view;
    mat4 u_proj;
    ivec2 u_mouse_pos;
    int highlight_idx;
    float _u_padding;
};

layout(set = 1, binding = 0) buffer Picking {
    int picking[];
};

layout(set = 2, binding = 0) uniform texture2D mask_texture;
layout(set = 2, binding = 1) uniform sampler mask_sampler;

layout(set = 3, binding = 0) uniform texture2D t_diffuse;
layout(set = 3, binding = 1) uniform sampler s_diffuse;

void main() {
    // get a value that correlates with the distance of the fragment from the camera
    float approx_z = gl_FragCoord.z;

    // object picking
    int pixel_x = int(gl_FragCoord.x);
    int pixel_y = int(gl_FragCoord.y);
    if (pixel_x == u_mouse_pos.x && pixel_y == u_mouse_pos.y) {
        atomicMin(picking[object_idx], floatBitsToInt(approx_z));
    }

    vec4 mask_color = texture(sampler2D(mask_texture, mask_sampler), v_uv_coords);

    // There is a mask to render with a wireframe effect. In order
    // to use it, we discard fragments based on mask alpha and a threshold
    float flatness = 0.001/fwidth(v_uv_coords.x) + 0.001/fwidth(v_uv_coords.y);
    float distance = 1/3 * approx_z;
    float threshold = distance + clamp(flatness, 0.0, 0.75);
    if (mask_color.a != 1.0 && mask_color.a <= threshold)
        discard;

    float mask_value = mask_color.r;
    mask_value = 0.5 * mask_value + 0.5;

    // matcap texturing
    highp vec4 scaled_normal = 0.49 * u_view * normalize(v_n_vector);
    highp vec2 muv = scaled_normal.xy + vec2(0.5,0.5);
    f_color = texture(sampler2D(t_diffuse, s_diffuse), vec2(muv.x, 1.0-muv.y));

    // final color
    float highlight_coeff = (object_idx == highlight_idx) ? 1.4 : 1.0;
    float z_light_coeff = 1 + v_n_vector.z * 0.2;
    f_color = z_light_coeff * highlight_coeff * mask_value * f_color;
    f_color.a = 1.0;

    // if you want to enable "printf debug", overwrite f_color before returning
    //f_color.rgb = v_n_vector.xyz;
}

