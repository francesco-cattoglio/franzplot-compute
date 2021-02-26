#version 450

layout(location=0) in vec3 in_position;
layout(location=1) in vec4 in_color;

layout(location=0) out vec4 color;

layout(set = 0, binding = 0) uniform Uniforms {
    mat4 u_view;
    mat4 u_proj;
    ivec2 u_mouse_pos;
    int highlight_id;
    float _u_padding;
};

void main() {
    color = in_color;
    mat4 changed_view;
    changed_view[0] = vec4(1.0, 0.0, 0.0, 0.0);
    //changed_view[0] = u_view[0];
    changed_view[1] = vec4(0.0, 1.0, 0.0, 0.0);
    changed_view[1] = u_view[2];
    changed_view[2] = vec4(0.0, 0.0, 1.0, 0.0);
    //changed_view[2] = u_view[2];

    // this shall never change
    //changed_view[3] = vec4(in_color.xyz, 1.0);
    changed_view[3] = u_view[3];
    gl_Position = u_proj * changed_view * vec4(in_position, 1.0);
    gl_Position += u_proj * u_view * vec4(in_color.xyz, 0.0);
}


