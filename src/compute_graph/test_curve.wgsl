[[block]]
struct Globals {
    t: f32;
    pi: f32;
};

[[block]]
struct InputBuffer {
    values: array<f32>;
};

[[block]]
struct OutputBuffer {
    positions: array<vec4<f32>>;
};

[[group(0), binding(0)]]
var<storage, read> input: InputBuffer;

[[group(0), binding(1)]]
var<storage, write> output: OutputBuffer;

[[group(1), binding(0)]]
var<uniform> globals: Globals;

[[stage(compute), workgroup_size(16)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
    let k = globals.t;
    let index = global_id.x;
    let t = input.values[index];
    output.positions[index] = vec4<f32>(t, t, t, 1.0);
}
