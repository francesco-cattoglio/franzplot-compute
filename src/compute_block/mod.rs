use anyhow::Result;
pub use smol_str::SmolStr;

pub mod point;
pub use point::{PointBlockDescriptor, PointData};

pub mod curve;
pub use curve::{CurveBlockDescriptor, CurveData};

pub mod surface;
pub use surface::{SurfaceData, SurfaceBlockDescriptor};

pub mod surface_renderer;
pub use surface_renderer::{SurfaceRendererData, SurfaceRendererBlockDescriptor};

pub mod interval;
pub use interval::{IntervalData, IntervalBlockDescriptor};

pub mod transform;
pub use transform::{TransformData, TransformBlockDescriptor};

pub mod matrix;
pub use matrix::{MatrixData, MatrixBlockDescriptor};

use serde::{Deserialize, Serialize};

pub enum BlockCreationError {
    Warning(&'static str),
    Error(&'static str),
}

pub enum ComputeBlock {
    Point(PointData),
    Interval(IntervalData),
    Curve(CurveData),
    Surface(SurfaceData),
    Transform(TransformData),
    Matrix(MatrixData),
    SurfaceRenderer(SurfaceRendererData),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: SmolStr,
    pub size: usize,
}
#[derive(Debug, Clone)]
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
    pub fn create_storage_buffer(&self, element_size: usize, device: &wgpu::Device) -> wgpu::Buffer {
        let buff_size = match self {
            Dimensions::D0 => element_size,
            Dimensions::D1(param)=> element_size * param.size,
            Dimensions::D2(par1, par2) => element_size * par1.size * par2.size,
        };
        device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            mapped_at_creation: false,
            size: buff_size as wgpu::BufferAddress,
            // TODO: vertex is actually only required for surface and curve renderers
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::MAP_READ,
        })
    }
}

impl ComputeBlock {
    pub fn get_buffer(&self) -> &wgpu::Buffer {
        match self {
            Self::Point(data) => &data.out_buffer,
            Self::Interval(data) => &data.out_buffer,
            Self::Curve(data) => &data.out_buffer,
            Self::Surface(data) => &data.out_buffer,
            Self::Transform(data) => &data.out_buffer,
            Self::Matrix(data) => &data.out_buffer,
            Self::SurfaceRenderer(data) => &data.vertex_buffer,
        }
    }

    pub fn get_dimensions(&self) -> &Dimensions {
        match self {
            Self::Point(data) => &data.out_dim,
            Self::Interval(data) => &data.out_dim,
            Self::Curve(data) => &data.out_dim,
            Self::Surface(data) => &data.out_dim,
            Self::Transform(data) => &data.out_dim,
            Self::Matrix(data) => &data.out_dim,
            Self::SurfaceRenderer(data) => &data.out_dim,
        }
    }

    pub fn encode(&self, globals_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
        match self {
            Self::Point(data) => data.encode(globals_bind_group, encoder),
            Self::Interval(data) => data.encode(globals_bind_group, encoder),
            Self::Curve(data) => data.encode(globals_bind_group, encoder),
            Self::Surface(data) => data.encode(globals_bind_group, encoder),
            Self::Transform(data) => data.encode(globals_bind_group, encoder),
            Self::Matrix(data) => data.encode(globals_bind_group, encoder),
            Self::SurfaceRenderer(data) => data.encode(encoder),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockDescriptor {
    pub id: String,
    pub data: DescriptorData,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum DescriptorData {
    Point (PointBlockDescriptor),
    Curve (CurveBlockDescriptor),
    Interval (IntervalBlockDescriptor),
    Surface (SurfaceBlockDescriptor),
    Matrix (MatrixBlockDescriptor),
    Transform (TransformBlockDescriptor),
    SurfaceRenderer (SurfaceRendererBlockDescriptor),
}

use crate::compute_chain::ComputeChain;
impl DescriptorData {
    pub fn to_block(&self, chain: &ComputeChain, device: &wgpu::Device) -> Result<ComputeBlock, BlockCreationError> {
        match &self {
            DescriptorData::Point(desc) => desc.to_block(&chain, device),
            DescriptorData::Curve(desc) => desc.to_block(&chain, device),
            DescriptorData::Interval(desc) => desc.to_block(&chain, device),
            DescriptorData::Surface(desc) => desc.to_block(&chain, device),
            DescriptorData::Matrix(desc) => desc.to_block(&chain, device),
            DescriptorData::Transform(desc) => desc.to_block(&chain, device),
            DescriptorData::SurfaceRenderer(desc) => desc.to_block(&chain, device),
        }
    }
}
