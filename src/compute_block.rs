use crate::compute_chain::ComputeChain;
use super::shader_processing::*;

use ultraviolet::Vec3u;

const LOCAL_SIZE_X: u32 = 16;
const LOCAL_SIZE_Y: u32 = 16;

pub enum ComputeBlock {
    Interval(IntervalData),
    Curve(CurveData),
    Surface(SurfaceData),
}

impl ComputeBlock {
    pub fn get_buffer(&self) -> &wgpu::Buffer {
        match self {
            Self::Interval(data) => &data.out_buffer,
            Self::Curve(data) => &data.out_buffer,
            Self::Surface(data) => &data.out_buffer,
        }
    }

    pub fn encode(&self, globals_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
        match self {
            Self::Interval(data) => data.encode(globals_bind_group, encoder),
            Self::Curve(data) => data.encode(globals_bind_group, encoder),
            Self::Surface(data) => data.encode(globals_bind_group, encoder),
        }
    }
}

#[derive(Debug)]
pub struct BlockDescriptor {
    pub id: String,
    pub data: DescriptorData,
}

#[derive(Debug)]
pub enum DescriptorData {
    Curve (CurveBlockDescriptor),
    Interval (IntervalBlockDescriptor),
    Surface (SurfaceBlockDescriptor),
}

#[derive(Debug)]
pub struct IntervalBlockDescriptor {
    pub begin: String,
    pub end: String,
    pub quality: u32,
    pub name: String,
}
impl IntervalBlockDescriptor {
    pub fn to_block(&self, chain: &ComputeChain, device: &wgpu::Device) -> ComputeBlock {
        ComputeBlock::Interval(IntervalData::new(chain, device, &self))
    }
}

pub struct IntervalData {
    out_buffer: wgpu::Buffer,
    buffer_size: wgpu::BufferAddress,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    out_sizes: Vec3u,
    name: String,
}

impl IntervalData {
    pub fn new(compute_chain: &ComputeChain, device: &wgpu::Device, descriptor: &IntervalBlockDescriptor) -> Self {
        let out_sizes = Vec3u::new(16 * descriptor.quality, 1, 1);
        let buffer_size = (out_sizes.x * std::mem::size_of::<f32>() as u32) as wgpu::BufferAddress;
        let out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: false,
            label: None,
            size: buffer_size,
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::STORAGE,
        });

        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = {dimy}) in;

layout(set = 0, binding = 0) buffer OutputBuffer {{
    float out_buff[];
}};

{globals_header}

void main() {{
    uint index = gl_GlobalInvocationID.x;
    float delta = ({interval_end} - {interval_begin}) / ({num_points} - 1.0);
    out_buff[index] = {interval_begin} + delta * index;
}}
"##, globals_header=&compute_chain.shader_header, interval_begin=&descriptor.begin, interval_end=&descriptor.end, num_points=out_sizes.x, dimx=LOCAL_SIZE_X, dimy=1
);
        println!("debug info for interval shader: \n{}", &shader_source);

        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for output buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer_slice: out_buffer.slice(..)
        });
        let (compute_pipeline, compute_bind_group) = compute_shader_from_glsl(shader_source.as_str(), &bindings, &compute_chain.globals_bind_layout, device, Some("Interval"));
        Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_sizes,
            buffer_size,
            name: descriptor.name.clone(),
        }
    }

    fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            compute_pass.dispatch((self.out_sizes.x/LOCAL_SIZE_X) as u32, 1, 1);
    }
}


#[derive(Debug)]
pub struct CurveBlockDescriptor {
    pub interval_input_id: String,
    pub x_function: String,
    pub y_function: String,
    pub z_function: String,
}
impl CurveBlockDescriptor {
    pub fn to_block(&self, chain: &ComputeChain, device: &wgpu::Device) -> ComputeBlock {
        ComputeBlock::Curve(CurveData::new(chain, device, &self))
    }
}

pub struct CurveData {
    out_buffer: wgpu::Buffer,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    out_sizes: Vec3u,
    #[allow(unused)]
    buffer_size: wgpu::BufferAddress,
}

impl CurveData {
    pub fn new(compute_chain: &ComputeChain, device: &wgpu::Device, descriptor: &CurveBlockDescriptor) -> Self {
        let interval_block = compute_chain.chain.get(&descriptor.interval_input_id).expect("unable to find dependency for curve block").clone();
        let interval_data: &IntervalData;
        if let ComputeBlock::Interval(data) = interval_block {
            interval_data = data;
        } else {
            panic!("internal error");
        }
        // We are creating a curve from an interval, output vertex count is the same as the input
        // one. Buffer size is 4 times as much, because we are storing a Vec4 instead of a f32
        let out_sizes = interval_data.out_sizes;
        let input_buffer_size = interval_data.buffer_size;
        let output_buffer_size = input_buffer_size * 4;
        let out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            mapped_at_creation: false,
            size: output_buffer_size,
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::STORAGE,
        });
        dbg!(&input_buffer_size);
        dbg!(&output_buffer_size);

        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = {dimy}) in;

layout(set = 0, binding = 0) buffer InputBuffer {{
    float {par}_buff[];
}};

layout(set = 0, binding = 1) buffer OutputBuffer {{
    vec4 out_buff[];
}};

{header}

void main() {{
    uint index = gl_GlobalInvocationID.x;
    float {par} = {par}_buff[index];
    out_buff[index].x = {fx};
    out_buff[index].y = {fy};
    out_buff[index].z = {fz};
    out_buff[index].w = 1;
}}
"##, header=&compute_chain.shader_header, par=&interval_data.name, dimx=LOCAL_SIZE_X, dimy=1, fx=&descriptor.x_function, fy=&descriptor.y_function, fz=&descriptor.z_function);
        println!("debug info for curve shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer_slice: interval_data.out_buffer.slice(..)
        });
        // add descriptor for output buffer
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer_slice: out_buffer.slice(..)
        });
        let (compute_pipeline, compute_bind_group) = compute_shader_from_glsl(shader_source.as_str(), &bindings, &compute_chain.globals_bind_layout, device, Some("Interval"));

        Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_sizes,
            buffer_size: output_buffer_size,
        }
    }

    fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            compute_pass.dispatch((self.out_sizes.x/LOCAL_SIZE_X) as u32, 1, 1);
    }
}

#[derive(Debug)]
pub struct SurfaceBlockDescriptor {
    pub interval_first_id: String,
    pub interval_second_id: String,
    pub x_function: String,
    pub y_function: String,
    pub z_function: String,
}
impl SurfaceBlockDescriptor {
    pub fn to_block(&self, chain: &ComputeChain, device: &wgpu::Device) -> ComputeBlock {
        ComputeBlock::Surface(SurfaceData::new(chain, device, &self))
    }
}

pub struct SurfaceData {
    out_buffer: wgpu::Buffer,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    out_sizes: Vec3u,
    #[allow(unused)]
    buffer_size: wgpu::BufferAddress,
}

impl SurfaceData {
    pub fn new(compute_chain: &ComputeChain, device: &wgpu::Device, descriptor: &SurfaceBlockDescriptor) -> Self {
        let first_interval_block = compute_chain.chain.get(&descriptor.interval_first_id).expect("unable to find dependency for curve block").clone();
        let first_interval_data: &IntervalData;
        if let ComputeBlock::Interval(data) = first_interval_block {
            first_interval_data = data;
        } else {
            panic!("internal error");
        }
        let second_interval_block = compute_chain.chain.get(&descriptor.interval_second_id).expect("unable to find dependency for curve block").clone();
        let second_interval_data: &IntervalData;
        if let ComputeBlock::Interval(data) = second_interval_block {
            second_interval_data = data;
        } else {
            panic!("internal error");
        }
        // We are creating a surface from 2 intervals, output vertex count is the product of the
        // two interval sizes. Buffer size is 4 times as much, because we are storing a Vec4
        let out_sizes = Vec3u { x: first_interval_data.out_sizes.x, y: second_interval_data.out_sizes.x, z: 1};
        let output_buffer_size = std::mem::size_of::<ultraviolet::Vec4>() as u64 * out_sizes.x as u64 * out_sizes.y as u64;
        let out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            mapped_at_creation: false,
            size: output_buffer_size as wgpu::BufferAddress,
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::MAP_READ,
        });
        dbg!(&output_buffer_size);

        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = {dimy}) in;

layout(set = 0, binding = 0) buffer InputBuffer1 {{
    float {par1}_buff[];
}};

layout(set = 0, binding = 1) buffer InputBuffer2 {{
    float {par2}_buff[];
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    vec4 out_buff[];
}};

{header}

void main() {{
    uint par1_idx = gl_GlobalInvocationID.x;
    uint par2_idx = gl_GlobalInvocationID.y;
    uint index = gl_GlobalInvocationID.x + gl_WorkGroupSize.x * gl_GlobalInvocationID.y;
    float {par1} = {par1}_buff[par1_idx];
    float {par2} = {par2}_buff[par2_idx];
    out_buff[index].x = {fx};
    out_buff[index].y = {fy};
    out_buff[index].z = {fz};
    out_buff[index].w = 1;
}}
"##, header=&compute_chain.shader_header, par1=&first_interval_data.name, par2=&second_interval_data.name, dimx=LOCAL_SIZE_X, dimy=LOCAL_SIZE_Y, fx=&descriptor.x_function, fy=&descriptor.y_function, fz=&descriptor.z_function);
        println!("debug info for curve shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffers
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer_slice: first_interval_data.out_buffer.slice(..)
        });
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer_slice: second_interval_data.out_buffer.slice(..)
        });
        // add descriptor for output buffer
        bindings.push(CustomBindDescriptor {
            position: 2,
            buffer_slice: out_buffer.slice(..)
        });
        let (compute_pipeline, compute_bind_group) = compute_shader_from_glsl(shader_source.as_str(), &bindings, &compute_chain.globals_bind_layout, device, Some("Interval"));

        Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_sizes,
            buffer_size: output_buffer_size,
        }
    }

    fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            compute_pass.dispatch((self.out_sizes.x/LOCAL_SIZE_X) as u32, (self.out_sizes.y/LOCAL_SIZE_Y) as u32, 1);
    }
}
