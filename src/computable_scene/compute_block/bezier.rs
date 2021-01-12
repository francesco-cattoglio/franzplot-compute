use crate::computable_scene::globals::Globals;
use crate::shader_processing::*;
use super::ComputeBlock;
use super::BlockId;
use super::Dimensions;
use super::BlockCreationError;
use super::{ProcessedMap, ProcessingResult};

#[derive(Debug)]
pub struct BezierBlockDescriptor {
    pub points: Vec<BlockId>,
    pub quality: usize,
}
impl BezierBlockDescriptor {
    pub fn make_block(self, device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap) -> ProcessingResult {
        Ok(ComputeBlock::Bezier(BezierData::new(device, globals, processed_blocks, self)?))
    }
}

pub struct BezierData {
    pub out_buffer: wgpu::Buffer,
    pub compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
    #[allow(unused)]
    buffer_size: wgpu::BufferAddress,
}

impl BezierData {
    pub fn new(device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap, descriptor: BezierBlockDescriptor) -> Result<Self, BlockCreationError> {
        unimplemented!();
    }
}
