[[block]]
struct Uniforms {
    view: mat4x4<f32>;
    proj: mat4x4<f32>;
    mouse_pos: vec2<f32>;
    highlight_id: u32;
    _padding: i32;
};

[[block]]
struct PickingBuffer {
    distances: array<f32>;
};

[[group(0), binding(0)]]
var<uniform> uniforms: Uniforms;

[[group(1), binding(0)]]
var<storage, read_write> picking: PickingBuffer;

[[group(2), binding(0)]]
var mask_texture: texture_2d<f32>;
[[group(2), binding(1)]]
var mask_sampler: sampler;

[[group(3), binding(0)]]
var diffuse_texture: texture_2d<f32>;
[[group(3), binding(1)]]
var diffuse_sampler: sampler;

struct MatcapVertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] uv_coords: vec2<f32>;
    [[location(1)]] normal: vec4<f32>;
    [[location(2)]] object_id: u32;
};

[[stage(vertex)]]
fn matcap_vs_main(
    [[builtin(instance_index)]] object_id: u32,
    [[location(0)]] position: vec4<f32>,
    [[location(1)]] normal: vec4<f32>,
    [[location(2)]] uv_coords: vec2<f32>,
    [[location(3)]] _padding: vec2<i32>
) -> MatcapVertexOutput {
    var out: MatcapVertexOutput;
    out.uv_coords = uv_coords;
    out.object_id = object_id;
    out.normal = normal;
    out.position = uniforms.proj * uniforms.view * position;
    return out;
}

// Matcap fragment shader
[[stage(fragment)]]
fn matcap_fs_main(in: MatcapVertexOutput) -> [[location(0)]] vec4<f32> {
    // read mask texture
    let mask_color = textureSample(mask_texture, mask_sampler, in.uv_coords);
    let darken_coeff: f32 = 0.5 * mask_color.r + 0.5;

    // color from matcap rexture
    let scaled_normal = 0.49 * (uniforms.view * normalize(in.normal));
    let matcap_uv = vec2<f32>(scaled_normal.x + 0.5, 0.5 - scaled_normal.y);
    let matcap_color = textureSample(diffuse_texture, diffuse_sampler, matcap_uv);

    // final color
    let highlight_coeff: f32 = select(1.0, 1.4, in.object_id == uniforms.highlight_id);
    let z_light_coeff: f32 = 1.0 + in.normal.z * 0.2;
    let final_color = vec4<f32>(z_light_coeff * highlight_coeff * darken_coeff * matcap_color.xyz, 1.0);
    return final_color;
}

struct ColorOnlyOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};

[[stage(vertex)]]
fn wireframe_vs_main(
    [[location(0)]] position: vec4<f32>,
    [[location(1)]] color: vec4<f32>,
) -> ColorOnlyOutput {
    var out: ColorOnlyOutput;
    out.position = uniforms.proj * uniforms.view * position;
    out.color = color;
    return out;
}

[[stage(vertex)]]
fn billboard_vs_main(
    [[location(0)]] position_2d: vec2<f32>,
    [[location(1)]] billboard_placement: vec3<f32>,
    [[location(2)]] color: vec4<f32>,
) -> ColorOnlyOutput {
    var out: ColorOnlyOutput;
    out.position = uniforms.proj * vec4<f32>(position_2d, 0.0, 1.0) + uniforms.proj * uniforms.view * vec4<f32>(billboard_placement, 0.0);
    return out;
}

[[stage(fragment)]]
fn color_fs_main(in: ColorOnlyOutput) -> [[location(0)]] vec4<f32> {
    return in.color;
}

