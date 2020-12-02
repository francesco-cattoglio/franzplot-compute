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

use super::Globals;
use crate::node_graph::{ Node, NodeContents, NodeGraph, };

pub type BlockId = i32;
pub type ProcessingResult = Result<ComputeBlock, BlockCreationError>;
pub type ProcessedMap = indexmap::IndexMap<BlockId, ProcessingResult>;

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
            // Beware: vertex is actually only required for surface renderer,
            // while copy and map are only needed when debugging/inspecting
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::MAP_READ,
        })
    }
}

impl ComputeBlock {
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

    pub fn from_node(device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap, node: &Node, graph: &NodeGraph) -> ProcessingResult {
        match *node.contents() {
            NodeContents::Interval {
                variable, begin, end, ..
            } => {
                let interval_descriptor = IntervalBlockDescriptor {
                    name: graph.get_attribute_as_string(variable).unwrap(),
                    begin: graph.get_attribute_as_string(begin).unwrap(),
                    end: graph.get_attribute_as_string(end).unwrap(),
                    quality: 4,
                };
                interval_descriptor.to_block(device, globals)
            },
            NodeContents::Point{
                x, y, z, ..
            } => {
                let point_descriptor = PointBlockDescriptor {
                    fx: graph.get_attribute_as_string(x).unwrap(),
                    fy: graph.get_attribute_as_string(y).unwrap(),
                    fz: graph.get_attribute_as_string(z).unwrap(),
                };
                point_descriptor.to_block(device, globals)
            },
            NodeContents::Curve {
                interval, fx, fy, fz, ..
            } => {
                let curve_descriptor = CurveBlockDescriptor {
                    interval: graph.get_attribute_as_linked_node(interval),
                    fx: graph.get_attribute_as_string(fx).unwrap(),
                    fy: graph.get_attribute_as_string(fy).unwrap(),
                    fz: graph.get_attribute_as_string(fz).unwrap(),
                };
                curve_descriptor.to_block(device, globals, processed_blocks)
            },
            NodeContents::Surface {
                interval_1, interval_2, fx, fy, fz, ..
            } => {
                let curve_descriptor = SurfaceBlockDescriptor {
                    interval_first: graph.get_attribute_as_linked_node(interval_1),
                    interval_second: graph.get_attribute_as_linked_node(interval_2),
                    fx: graph.get_attribute_as_string(fx).unwrap(),
                    fy: graph.get_attribute_as_string(fy).unwrap(),
                    fz: graph.get_attribute_as_string(fz).unwrap(),
                };
                curve_descriptor.to_block(device, globals, processed_blocks)
            },
            NodeContents::Transform {
                geometry, matrix, ..
            } => {
                let transform_descriptor = TransformBlockDescriptor {
                    geometry: graph.get_attribute_as_linked_node(geometry),
                    matrix: graph.get_attribute_as_linked_node(matrix),
                };
                transform_descriptor.to_block(device, processed_blocks)
            },
            NodeContents::Matrix {
                interval, row_1, row_2, row_3, ..
            } => {
                use std::convert::TryInto;
                let matrix_descriptor = MatrixBlockDescriptor {
                    interval: graph.get_attribute_as_linked_node(interval),
                    row_1: graph.get_attribute_as_matrix_row(row_1).unwrap(),
                    row_2: graph.get_attribute_as_matrix_row(row_2).unwrap(),
                    row_3: graph.get_attribute_as_matrix_row(row_3).unwrap(),
                };
                matrix_descriptor.to_block(device, globals, processed_blocks)
            },
            NodeContents::Rendering {
                geometry,
            } => {
                let rendering_descriptor = RenderingBlockDescriptor {
                    geometry: graph.get_attribute_as_linked_node(geometry)
                };
                rendering_descriptor.to_block(device, processed_blocks)
            },
            NodeContents::Group => {
                unimplemented!()
            }
        }
    }
}


