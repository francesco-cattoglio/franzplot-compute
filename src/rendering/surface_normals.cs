#version 450
layout(local_size_x = 16, local_size_y = 16) in;

layout(set = 0, binding = 0) buffer InputVertices {
    vec4 in_buff[];
};

layout(set = 0, binding = 1) buffer OutputData {
    vec4 out_buff[];
};


void main() {
    // this shader prepares the data for surface rendering.
    // output data will have the following format:
    // for each vertex, we have a vec4 representing the position,
    // then a vec4 representing the normal

    // normal computation is done computing the tangent and cotangent of the surface via finite differences
    // and then crossing the two vectors.
    uint x_size = gl_NumWorkGroups.x * gl_WorkGroupSize.x;
    uint y_size = gl_NumWorkGroups.y * gl_WorkGroupSize.y;

    // I still need to test how bad the performance can be when branching inside a compute shader.
    uint i = gl_GlobalInvocationID.x;
    uint j = gl_GlobalInvocationID.y;
    uint idx = i + j * x_size;
    vec3 x_tangent;
    if (i == 0) {
        x_tangent = (-1.5*in_buff[idx] + 2.0*in_buff[idx+1] - 0.5*in_buff[idx+2]).xyz;
    } else if (i == x_size-1) {
        x_tangent = ( 1.5*in_buff[idx] - 2.0*in_buff[idx-1] + 0.5*in_buff[idx-2]).xyz;
    } else {
        x_tangent = (-0.5*in_buff[idx-1] + 0.5*in_buff[idx+1]).xyz;
    }
    vec3 y_tangent;
    if (j == 0) {
        y_tangent = (-1.5*in_buff[idx] + 2.0*in_buff[idx+x_size] - 0.5*in_buff[idx+2*x_size]).xyz;
    } else if (j == y_size-1) {
        y_tangent = ( 1.5*in_buff[idx] - 2.0*in_buff[idx-x_size] + 0.5*in_buff[idx-2*x_size]).xyz;
    } else {
        y_tangent = (-0.5*in_buff[idx-x_size] + 0.5*in_buff[idx+x_size]).xyz;
    }

    vec3 crossed = cross(y_tangent, x_tangent);
    float len = length(crossed);
    vec3 normal = (len > 1e-4) ? 1.0/len*crossed : vec3(0.0, 0.0, 0.0);
    out_buff[idx*3] = in_buff[idx];
    out_buff[idx*3+1] = vec4(normal.xyz, 0.0);
    out_buff[idx*3+2] = vec4(i/(x_size-1.0), j/(y_size-1.0), 0.0, 0.0);
}
