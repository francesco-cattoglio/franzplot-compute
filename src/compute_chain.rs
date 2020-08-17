use crate::compute_block::*;
use anyhow::{Result, anyhow};

use std::collections::BTreeMap;

pub struct ComputeChain {
    pub chain: BTreeMap<String, ComputeBlock>,
    globals_buffer: wgpu::Buffer,
    pub globals_bind_layout: wgpu::BindGroupLayout,
    pub globals_bind_group: wgpu::BindGroup,
    pub shader_header: String,
}

pub struct Context {
    pub globals: BTreeMap<String, f32>,
}

impl ComputeChain {
    fn new(device: &wgpu::Device, context: &Context) -> Self {
        let globals = &context.globals;
        let chain = BTreeMap::<String, ComputeBlock>::new();
        let buffer_size = (globals.len() * std::mem::size_of::<f32>()) as wgpu::BufferAddress;

        let values: Vec<f32> = globals.values().copied().collect();
        let globals_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&values),
            wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM
        );
        let globals_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::UniformBuffer {
                            dynamic: false,
                        }
                    },
                ],
                label: Some("Variables uniform layout")
            });
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &globals_bind_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &globals_buffer,
                        range: 0.. buffer_size,
                    },
                },
            ],
            label: Some("variables bind group")
        });

        let mut shader_header = String::new();
        shader_header.push_str(r##"
layout(set = 1, binding = 0) uniform Uniforms {
"##);
        for var_name in globals.keys() {
            shader_header.push_str(format!("\tfloat {};\n", var_name).as_str());
        }
        shader_header.push_str(r##"};
"##);
        println!("debug info for shader header: {}", &shader_header);
        Self {
            chain,
            shader_header,
            globals_bind_layout,
            globals_bind_group,
            globals_buffer,
        }
    }

    pub fn run_chain(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder this time"),
        });
        for block in self.chain.values() {
            block.encode(&self.globals_bind_group, &mut encoder);
        }
        let compute_queue = encoder.finish();
        queue.submit(&[compute_queue]);
    }

    pub fn insert(&mut self, id: &String, block: ComputeBlock) -> Result<()> {
        if self.chain.contains_key(id) {
            Err(anyhow!("Tried to insert two blocks that had the same id"))
        } else {
            self.chain.insert(id.clone(), block);
            Ok(())
        }
    }

    pub fn create_from_descriptors(device: &wgpu::Device, descriptors: Vec<BlockDescriptor>, globals: Context) -> Result<Self> {
        let mut chain = Self::new(device, &globals);
        // right now descriptors need to be in the "correct" order, so that all blocks that depend
        // on something are encountered after the blocks they depend on.
        for descriptor in descriptors.iter() {
            let block: ComputeBlock = match &descriptor.data {
                DescriptorData::Curve(desc) => desc.to_block(&chain, device),
                DescriptorData::Interval(desc) => ComputeBlock::Interval(IntervalData::new(&chain, device, desc)),
            };
            chain.insert(&descriptor.id, block)?;
        }

        return Ok(chain);
    }
}


