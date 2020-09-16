use anyhow::Result;
pub use smol_str::SmolStr;

pub mod curve;
pub use curve::{CurveBlockDescriptor, CurveData};

pub mod surface;
pub use surface::{SurfaceData, SurfaceBlockDescriptor};

pub mod interval;
pub use interval::{IntervalData, IntervalBlockDescriptor};

pub mod transform;
pub use transform::{TransformData, TransformBlockDescriptor};

pub mod matrix;
pub use matrix::{MatrixData, MatrixBlockDescriptor};

pub enum ComputeBlock {
    Interval(IntervalData),
    Curve(CurveData),
    Surface(SurfaceData),
    Transform(TransformData),
    Matrix(MatrixData),
}

#[derive(Clone)]
pub struct Parameter {
    name: SmolStr,
    size: usize,
}
#[derive(Clone)]
pub enum Dimensions {
    D0,
    D1(Parameter),
    D2(Parameter, Parameter)
}

impl Dimensions {

    pub fn as_0d(&self) -> Result<()> {
        match self {
            Self::D0 => Ok(()),
            _ => Err(anyhow::anyhow!("error converting dimensions to 0D")),
        }
    }
    pub fn as_1d(&self) -> Result<Parameter> {
        match self {
            Self::D1(dim) => Ok(dim.clone()),
            _ => Err(anyhow::anyhow!("error converting dimensions to 1D")),
        }
    }
    pub fn as_2d(&self) -> Result<(Parameter, Parameter)> {
        match self {
            Self::D2(dim1, dim2) => Ok((dim1.clone(), dim2.clone())),
            _ => Err(anyhow::anyhow!("error converting dimensions to 2D")),
        }
    }
}

impl ComputeBlock {
    pub fn get_buffer(&self) -> &wgpu::Buffer {
        match self {
            Self::Interval(data) => &data.out_buffer,
            Self::Curve(data) => &data.out_buffer,
            Self::Surface(data) => &data.out_buffer,
            Self::Transform(data) => &data.out_buffer,
            Self::Matrix(data) => &data.out_buffer,
        }
    }

    pub fn encode(&self, globals_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
        match self {
            Self::Interval(data) => data.encode(globals_bind_group, encoder),
            Self::Curve(data) => data.encode(globals_bind_group, encoder),
            Self::Surface(data) => data.encode(globals_bind_group, encoder),
            Self::Transform(data) => data.encode(globals_bind_group, encoder),
            Self::Matrix(data) => data.encode(globals_bind_group, encoder),
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
    Matrix (MatrixBlockDescriptor),
}

use crate::compute_chain::ComputeChain;
impl DescriptorData {
    pub fn to_block(&self, chain: &ComputeChain, device: &wgpu::Device) -> ComputeBlock {
        match &self {
            DescriptorData::Curve(desc) => desc.to_block(&chain, device),
            DescriptorData::Interval(desc) => desc.to_block(&chain, device),
            DescriptorData::Surface(desc) => desc.to_block(&chain, device),
            DescriptorData::Matrix(desc) => desc.to_block(&chain, device),
        }
    }
}
