use ultraviolet::Vec3u;

pub mod curve;
pub use curve::{CurveBlockDescriptor, CurveData};

pub mod surface;
pub use surface::{SurfaceData, SurfaceBlockDescriptor};

pub mod interval;
pub use interval::{IntervalData, IntervalBlockDescriptor};

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


