[[block]]
struct Globals {
};

let pi: f32 = 3.1415;

[[group(0), binding(0)]]
var<uniform> _globals: Globals;

[[block]]
struct OutputBuffer {
    values: array<f32>;
};

[[group(1), binding(0)]]
var<storage, write> _output: OutputBuffer;

[[stage(compute), workgroup_size(256)]]
fn main([[builtin(global_invocation_id)]] _global_id: vec3<u32>) {
    let _index = _global_id.x;
    let _delta: f32 = (10.0 - -10.0) / (256.0 - 1.0);
    _output.values[_index] = (-10.0) + _delta * f32(_index);
}
