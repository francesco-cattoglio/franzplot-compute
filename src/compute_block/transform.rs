use crate::compute_chain::ComputeChain;
use crate::shader_processing::*;
use super::ComputeBlock;
use ultraviolet::Vec3u;

#[derive(Debug)]
pub struct TransformBlockDescriptor {
    pub geometry_id: String,
    pub matrix_id: String,
}

impl TransformBlockDescriptor {
    pub fn to_block(&self, chain: &ComputeChain, device: &wgpu::Device) -> ComputeBlock {
        ComputeBlock::Transform(TransformData::new(chain, device, &self))
    }
}

pub struct TransformData {
    pub out_buffer: wgpu::Buffer,
    pub buffer_size: wgpu::BufferAddress,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_sizes: Vec3u,
}

impl TransformData {
    pub fn new(compute_chain: &ComputeChain, device: &wgpu::Device, descriptor: &TransformBlockDescriptor) -> Self {
        let geometry_block = compute_chain.chain.get(&descriptor.geometry_id).expect("could not find input geometry");
        let matrix_block = compute_chain.chain.get(&descriptor.matrix_id).expect("could not find input matrix");
        let geometry_out_sizes = match geometry_block {
            ComputeBlock::Curve(data) => data.out_sizes,
            ComputeBlock::Surface(data) => data.out_sizes,
            _ => panic!("Internal error"),
        };
        let out_sizes = Vec3u { x: geometry_out_sizes.x, y: geometry_out_sizes.y, z: 1};
        let output_buffer_size = std::mem::size_of::<ultraviolet::Vec4>() as u64 * out_sizes.x as u64 * out_sizes.y as u64;
        let out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            mapped_at_creation: false,
            size: output_buffer_size as wgpu::BufferAddress,
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::MAP_READ,
        });
        let matrix_buffer = 
        unimplemented!();
    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
        unimplemented!();
    }

}
