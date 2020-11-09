use anyhow::Result;
pub use smol_str::SmolStr;

pub mod point;
pub use point::{PointBlockDescriptor, PointData};

pub mod curve;
pub use curve::{CurveBlockDescriptor, CurveData};

pub mod surface;
pub use surface::{SurfaceData, SurfaceBlockDescriptor};

pub mod rendering;
pub use rendering::{RenderingData, RenderingBlockDescriptor};

pub mod interval;
pub use interval::{IntervalData, IntervalBlockDescriptor};

pub mod transform;
pub use transform::{TransformData, TransformBlockDescriptor};

pub mod matrix;
pub use matrix::{MatrixData, MatrixBlockDescriptor};

use serde::{Deserialize, Serialize};

pub type BlockId = i32;
pub type ProcessingResult = Result<ComputeBlock, BlockCreationError>;
pub type ProcessedMap = indexmap::IndexMap<BlockId, ProcessingResult>;

use crate::compute_chain::Globals;

#[derive(Debug, Clone)]
pub enum BlockCreationError {
    IncorrectAttributes(&'static str),
    InputMissing(&'static str),
    InputInvalid(&'static str),
    InputNotBuilt(&'static str),
    InternalError(&'static str),
}

pub enum ComputeBlock {
    Point(PointData),
    Interval(IntervalData),
    Curve(CurveData),
    Surface(SurfaceData),
    Transform(TransformData),
    Matrix(MatrixData),
    Rendering(RenderingData),
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
    #[allow(unused)]
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
            // TODO: vertex is actually only required for surface renderer,
            // while copy and map are only needed when debugging/inspecting
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::MAP_READ,
        })
    }
}

//TODO: maybe remove the get_buffer and get_dimensions functions.
//Whenever we use a computeblock we have to match on its type anyway
impl ComputeBlock {
    #[allow(unused)]
    pub fn get_dimensions(&self) -> &Dimensions {
        match self {
            Self::Point(data) => &data.out_dim,
            Self::Interval(data) => &data.out_dim,
            Self::Curve(data) => &data.out_dim,
            Self::Surface(data) => &data.out_dim,
            Self::Transform(data) => &data.out_dim,
            Self::Matrix(data) => &data.out_dim,
            Self::Rendering(data) => &data.out_dim,
        }
    }

    pub fn encode(&self, globals_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
        match self {
            Self::Point(data) => data.encode(globals_bind_group, encoder),
            Self::Interval(data) => data.encode(globals_bind_group, encoder),
            Self::Curve(data) => data.encode(globals_bind_group, encoder),
            Self::Surface(data) => data.encode(globals_bind_group, encoder),
            Self::Matrix(data) => data.encode(globals_bind_group, encoder),
            Self::Transform(data) => data.encode(encoder),
            Self::Rendering(data) => data.encode(encoder),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockDescriptor {
    pub id: BlockId,
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
    Rendering (RenderingBlockDescriptor),
}

impl DescriptorData {
    pub fn get_input_ids(&self) -> Vec<BlockId> {
        match &self {
            // we know that some nodes have no input at all, so we can always return an empty vec
            DescriptorData::Point(_) => vec![],
            DescriptorData::Interval(_) => vec![],
            DescriptorData::Curve(desc) => desc.get_input_ids(),
            DescriptorData::Surface(desc) => desc.get_input_ids(),
            DescriptorData::Matrix(desc) => desc.get_input_ids(),
            DescriptorData::Transform(desc) => desc.get_input_ids(),
            DescriptorData::Rendering(desc) => desc.get_input_ids(),
        }
    }

    // Not all the blocks require the same inputs at creation time.
    // As an example, a Transform block does not use any global variable,
    // while an Interval cannot depend on any other already-processed block
    pub fn to_block(&self, device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap) -> ProcessingResult {
        match &self {
            DescriptorData::Point(desc) => desc.to_block(device, globals),
            DescriptorData::Interval(desc) => desc.to_block(device, globals),
            DescriptorData::Curve(desc) => desc.to_block(device, globals, processed_blocks),
            DescriptorData::Surface(desc) => desc.to_block(device, globals, processed_blocks),
            DescriptorData::Matrix(desc) => desc.to_block(device, globals, processed_blocks),
            DescriptorData::Transform(desc) => desc.to_block(device, processed_blocks),
            DescriptorData::Rendering(desc) => desc.to_block(device, processed_blocks),
        }
    }
}
