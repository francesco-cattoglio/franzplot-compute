pub use smol_str::SmolStr;

pub mod vector;
pub use vector::{VectorBlockDescriptor, VectorData};

pub mod point;
pub use point::{PointBlockDescriptor, PointData};

pub mod bezier;
pub use bezier::{BezierBlockDescriptor, BezierData};

pub mod curve;
pub use curve::{CurveBlockDescriptor, CurveData};

pub mod plane;
pub use plane::{PlaneBlockDescriptor};

pub mod surface;
pub use surface::{SurfaceData, SurfaceBlockDescriptor};

pub mod rendering;
pub use rendering::{RenderingData, RenderingBlockDescriptor};

pub mod vector_rendering;
pub use vector_rendering::{VectorRenderingData, VectorRenderingBlockDescriptor};

pub mod interval;
pub use interval::{IntervalData, IntervalBlockDescriptor};

pub mod sample;
pub use sample::{SampleData, SampleBlockDescriptor};

pub mod transform;
pub use transform::{TransformData, TransformBlockDescriptor};

pub mod matrix;
pub use matrix::{MatrixData, MatrixBlockDescriptor};

pub mod prefab;
pub use prefab::{PrefabData, PrefabBlockDescriptor};

use super::Globals;
use crate::rendering::model::Model;
use crate::node_graph::{ Node, NodeContents, NodeGraph, };

pub type BlockId = i32;
pub type PrefabId = i32;
pub type ProcessingResult = Result<ComputeBlock, BlockCreationError>;
pub type ProcessedMap = indexmap::IndexMap<BlockId, ProcessingResult>;

#[derive(Debug, Clone)]
pub enum BlockCreationError {
    IncorrectAttributes(&'static str),
    InputMissing(&'static str),
    InputInvalid(&'static str),
    InputNotBuilt(&'static str),
    InternalError(String),
    IncorrectExpression(String),
}

pub enum ComputeBlock {
    Vector(VectorData),
    Point(PointData),
    Interval(IntervalData),
    Sample(SampleData),
    Bezier(BezierData),
    Curve(CurveData),
    Surface(SurfaceData),
    Transform(TransformData),
    Matrix(MatrixData),
    Rendering(RenderingData),
    VectorRendering(VectorRenderingData),
    Prefab(PrefabData),
}

/// a parameter can be anonymous, e.g. when created by a Bezier node
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: Option<String>,
    pub size: usize,
    pub begin: String,
    pub end: String,
    pub use_interval_as_uv: bool,
}

impl Parameter {
    pub fn is_equal(&self, other: &Parameter) -> Result <bool, BlockCreationError> {
        match (&self.name, &other.name) {
            (None, None) => Ok(false),
            (None, Some(_)) => Ok(false),
            (Some(_), None) => Ok(false),
            (Some(self_name), Some(other_name)) => {
                if self_name == other_name {
                    // having the same name but a different quality, begin or end attribute is an error.
                    if self.size != other.size {
                        Err(BlockCreationError::IncorrectAttributes(" The input intervals \n have the same name \n but different 'quality' "))
                    } else if self.begin != other.begin {
                        Err(BlockCreationError::IncorrectAttributes(" The input intervals \n have the same name \n but different 'begin' "))
                    } else if self.end != other.end {
                        Err(BlockCreationError::IncorrectAttributes(" The input intervals \n have the same name \n but different 'end' "))
                    } else {
                        Ok(true)
                    }
                } else {
                    Ok(false)
                }

            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Dimensions {
    D0,
    D1(Parameter),
    D2(Parameter, Parameter),
    D3(usize, PrefabId),
}

impl Dimensions {
    pub fn as_0d(&self) -> Result<(), BlockCreationError> {
        match self {
            Self::D0 => Ok(()),
            Self::D1(_) => Err(BlockCreationError::InternalError("Dimensions mismatch: called `as_0d()` on a 1D object".into())),
            Self::D2(_, _) => Err(BlockCreationError::InternalError("Dimensions mismatch: called `as_0d()` on a 2D object".into())),
            Self::D3(_, _) => Err(BlockCreationError::InternalError("Dimensions mismatch: called `as_0d()` on a 3D object".into())),
        }
    }
    pub fn as_1d(&self) -> Result<Parameter, BlockCreationError> {
        match self {
            Self::D0 => Err(BlockCreationError::InternalError("Dimensions mismatch: called `as_1d()` on a 0D object".into())),
            Self::D1(dim) => Ok(dim.clone()),
            Self::D2(_, _) => Err(BlockCreationError::InternalError("Dimensions mismatch: called `as_1d()` on a 2D object".into())),
            Self::D3(_, _) => Err(BlockCreationError::InternalError("Dimensions mismatch: called `as_1d()` on a 3D object".into())),
        }
    }
    pub fn as_2d(&self) -> Result<(Parameter, Parameter), BlockCreationError> {
        match self {
            Self::D0 => Err(BlockCreationError::InternalError("Dimensions mismatch: called `as_2d()` on a 0D object".into())),
            Self::D1(_) => Err(BlockCreationError::InternalError("Dimensions mismatch: called `as_2d()` on a 1D object".into())),
            Self::D2(dim1, dim2) => Ok((dim1.clone(), dim2.clone())),
            Self::D3(_, _) => Err(BlockCreationError::InternalError("Dimensions mismatch: called `as_2d()` on a 3D object".into())),
        }
    }
    pub fn as_3d(&self) -> Result<(usize, PrefabId), BlockCreationError> {
        match self {
            Self::D0 => Err(BlockCreationError::InternalError("Dimensions mismatch: called `as_3d()` on a 0D object".into())),
            Self::D1(_) => Err(BlockCreationError::InternalError("Dimensions mismatch: called `as_3d()` on a 1D object".into())),
            Self::D2(_, _) => Err(BlockCreationError::InternalError("Dimensions mismatch: called `as_3d()` on a 2D object".into())),
            Self::D3(vertex_count, index_buffer) => Ok((*vertex_count, *index_buffer)),
        }
    }
    pub fn create_storage_buffer(&self, element_size: usize, device: &wgpu::Device) -> wgpu::Buffer {
        let buff_size = match self {
            Dimensions::D0 => element_size,
            Dimensions::D1(param)=> element_size * param.size,
            Dimensions::D2(par1, par2) => element_size * par1.size * par2.size,
            // since 3D geometry can only be created by loading it, one should never attempt to
            // create a buffer from it
            Dimensions::D3(vertex_count, _)=> element_size * vertex_count,
        };
        device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            mapped_at_creation: false,
            size: buff_size as wgpu::BufferAddress,
            // Beware:copy and map are only needed when debugging/inspecting
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::MAP_READ,
        })
    }
}

impl ComputeBlock {
    pub fn encode(&self, globals_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
        match self {
            Self::Vector(data) => data.encode(globals_bind_group, encoder),
            Self::Point(data) => data.encode(globals_bind_group, encoder),
            Self::Interval(data) => data.encode(globals_bind_group, encoder),
            Self::Sample(data) => data.encode(globals_bind_group, encoder),
            Self::Bezier(data) => data.encode(encoder),
            Self::Curve(data) => data.encode(globals_bind_group, encoder),
            Self::Surface(data) => data.encode(globals_bind_group, encoder),
            Self::Matrix(data) => data.encode(globals_bind_group, encoder),
            Self::Transform(data) => data.encode(encoder),
            Self::Rendering(data) => data.encode(globals_bind_group, encoder),
            Self::VectorRendering(data) => data.encode(encoder),
            Self::Prefab(data) => data.encode(globals_bind_group, encoder),
        }
    }

    pub fn from_node(device: &wgpu::Device, models: &[Model], globals: &Globals, processed_blocks: &ProcessedMap, node: &Node, graph: &NodeGraph) -> ProcessingResult {
        match *node.contents() {
            NodeContents::Interval {
                variable, begin, end, quality, ..
            } => {
                let interval_descriptor = IntervalBlockDescriptor {
                    name: graph.get_attribute_as_string(variable).unwrap(),
                    begin: graph.get_attribute_as_string(begin).unwrap(),
                    end: graph.get_attribute_as_string(end).unwrap(),
                    quality: graph.get_attribute_as_usize(quality).unwrap(),
                };
                interval_descriptor.make_block(device, globals)
            },
            NodeContents::Sample {
                geometry, parameter, value, ..
            } => {
                let sample_descriptor = SampleBlockDescriptor {
                    geometry: graph.get_attribute_as_linked_node(geometry),
                    parameter: graph.get_attribute_as_string(parameter).unwrap(),
                    value: graph.get_attribute_as_string(value).unwrap(),
                };
                sample_descriptor.make_block(device, globals, processed_blocks)
            },
            NodeContents::Vector {
                x, y, z, ..
            } => {
                let vector_descriptor = VectorBlockDescriptor {
                    vx: graph.get_attribute_as_string(x).unwrap(),
                    vy: graph.get_attribute_as_string(y).unwrap(),
                    vz: graph.get_attribute_as_string(z).unwrap(),
                };
                vector_descriptor.make_block(device, globals)
            },
            NodeContents::Point {
                x, y, z, ..
            } => {
                let point_descriptor = PointBlockDescriptor {
                    fx: graph.get_attribute_as_string(x).unwrap(),
                    fy: graph.get_attribute_as_string(y).unwrap(),
                    fz: graph.get_attribute_as_string(z).unwrap(),
                };
                point_descriptor.make_block(device, globals)
            },
            NodeContents::Bezier {
                p0, p1, p2, p3, quality, ..
            } => {
                use crate::node_graph::NodeID;
                let mut points = Vec::<NodeID>::new();
                if let Some(id) = graph.get_attribute_as_linked_node(p0) {
                    points.push(id);
                }
                if let Some(id) = graph.get_attribute_as_linked_node(p1) {
                    points.push(id);
                }
                if let Some(id) = graph.get_attribute_as_linked_node(p2) {
                    points.push(id);
                }
                if let Some(id) = graph.get_attribute_as_linked_node(p3) {
                    points.push(id);
                }
                let bezier_descriptor = BezierBlockDescriptor {
                    points,
                    quality: graph.get_attribute_as_usize(quality).unwrap(),
                };
                bezier_descriptor.make_block(device, processed_blocks)
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
                curve_descriptor.make_block(device, globals, processed_blocks)
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
                curve_descriptor.make_block(device, globals, processed_blocks)
            },
            NodeContents::Plane {
                center, normal, size, ..
            } => {
                let plane_descriptor = PlaneBlockDescriptor {
                    center: graph.get_attribute_as_linked_node(center),
                    normal: graph.get_attribute_as_linked_node(normal),
                    side_length: graph.get_attribute_as_usize(size).unwrap(),
                };
                plane_descriptor.make_block(device, processed_blocks)
            },
            NodeContents::Transform {
                geometry, matrix, ..
            } => {
                let transform_descriptor = TransformBlockDescriptor {
                    geometry: graph.get_attribute_as_linked_node(geometry),
                    matrix: graph.get_attribute_as_linked_node(matrix),
                };
                transform_descriptor.make_block(device, processed_blocks)
            },
            NodeContents::Matrix {
                interval, row_1, row_2, row_3, ..
            } => {
                let matrix_descriptor = MatrixBlockDescriptor {
                    interval: graph.get_attribute_as_linked_node(interval),
                    row_1: graph.get_attribute_as_matrix_row(row_1).unwrap(),
                    row_2: graph.get_attribute_as_matrix_row(row_2).unwrap(),
                    row_3: graph.get_attribute_as_matrix_row(row_3).unwrap(),
                };
                matrix_descriptor.make_block(device, globals, processed_blocks)
            },
            NodeContents::RotationMatrix {
                axis, angle, ..
            } => {
                let axis = graph.get_attribute_as_axis(axis).unwrap();
                let angle = graph.get_attribute_as_string(angle).unwrap();
                let matrix_descriptor = MatrixBlockDescriptor::new_from_rotation(globals, axis, angle)?;
                matrix_descriptor.make_block(device, globals, processed_blocks)
            },
            NodeContents::TranslationMatrix {
                vector, ..
            } => {
                let maybe_id = graph.get_attribute_as_linked_node(vector);
                let node_id = match maybe_id {
                    Some(id) => id,
                    None => return Err(BlockCreationError::InputMissing(" this Translation Matrix node \n is missing the vector input ")),
                };
                // TODO: this method of getting the vector input is a temporary hacked in solution.
                // If any vector processing capabilities gets added to the graph node, make sure to
                // change this code.
                let vector_node = graph.get_node(node_id).unwrap();
                match vector_node.contents {
                    NodeContents::Vector { x, y, z, .. } => {
                        let x = graph.get_attribute_as_string(x).unwrap();
                        let y = graph.get_attribute_as_string(y).unwrap();
                        let z = graph.get_attribute_as_string(z).unwrap();

                        let matrix_descriptor = MatrixBlockDescriptor::new_from_translation(globals, x, y, z)?;
                        matrix_descriptor.make_block(device, globals, processed_blocks)
                    },
                    _ => Err(BlockCreationError::InputInvalid(" the input is not a vector node "))
                }
            },
            NodeContents::Rendering {
                geometry, thickness, mask, material,
            } => {
                let rendering_descriptor = RenderingBlockDescriptor {
                    geometry: graph.get_attribute_as_linked_node(geometry),
                    mask: graph.get_attribute_as_usize(mask).unwrap(),
                    material: graph.get_attribute_as_usize(material).unwrap(),
                    thickness: graph.get_attribute_as_usize(thickness).unwrap(),
                };
                rendering_descriptor.make_block(device, globals, models, processed_blocks)
            },
            NodeContents::VectorRendering {
                application_point, vector, thickness, material,
            } => {
                let rendering_descriptor = VectorRenderingBlockDescriptor {
                    application_point: graph.get_attribute_as_linked_node(application_point),
                    vector: graph.get_attribute_as_linked_node(vector),
                    material: graph.get_attribute_as_usize(material).unwrap(),
                    thickness: graph.get_attribute_as_usize(thickness).unwrap(),
                };
                rendering_descriptor.make_block(device, processed_blocks)
            },
            NodeContents::Primitive {
                primitive, size, ..
            } => {
                let prefab_descriptor = PrefabBlockDescriptor {
                    size: graph.get_attribute_as_string(size).unwrap(),
                    prefab_id: graph.get_attribute_as_usize(primitive).unwrap() as i32,
                };
                prefab_descriptor.make_block(device, models, globals)
            },
            NodeContents::Group => {
                todo!()
            }
        }
    }
}


