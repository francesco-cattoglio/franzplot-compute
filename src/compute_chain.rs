use crate::compute_block::*;
use anyhow::{Result, anyhow};

use std::collections::hash_map::HashMap;
use std::rc::Rc;
pub struct ComputeChain {
    pub blocks: HashMap<u16, Rc<dyn ComputeBlock>>,
    variables_buffer: wgpu::Buffer,
    pub variables_bind_layout: wgpu::BindGroupLayout,
    pub variables_bind_group: wgpu::BindGroup,
    variables_names: Vec<String>,
    pub shader_header: String,
}

#[derive(Debug)]
pub enum BlockDescriptor {
    Curve (CurveBlockDescriptor),
    Interval (IntervalBlockDescriptor),
}

pub struct Context {
    pub var_names: Vec<String>,
}

impl ComputeChain {
    fn new(device: &wgpu::Device, variables: &Context) -> Self {

        let blocks = HashMap::<u16, Rc<dyn ComputeBlock>>::new();
        let buffer_size = (variables.var_names.len() * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        //let variables_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        //    label: None,
        //    size: buffer_size,
        //    usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM,
        //});

        let variables_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&[3.0f32, 0.13f32]),
            wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM
            );
        let variables_bind_layout =
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
        let variables_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &variables_bind_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &variables_buffer,
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
        for var_name in &variables.var_names {
            shader_header.push_str(format!("\tfloat {};\n", var_name).as_str());
        }
        shader_header.push_str(r##"};
"##);
        println!("{}", &shader_header);
        Self {
            blocks,
            shader_header,
            variables_bind_layout,
            variables_bind_group,
            variables_buffer,
            variables_names: variables.var_names.clone()
        }
    }

    pub fn run_chain(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder this time"),
        });
        self.blocks[&1].encode(&self.variables_bind_group, &mut encoder);
        let compute_queue = encoder.finish();
        queue.submit(&[compute_queue]);
    }

    pub fn insert(&mut self, id: u16, block: Rc<dyn ComputeBlock>) -> Result<()> {
        if self.blocks.contains_key(&id) {
            Err(anyhow!("a"))
        } else {
            self.blocks.insert(id, block);
            Ok(())
        }
    }
    pub fn create_from_descriptors(device: &wgpu::Device, descriptors: Vec<BlockDescriptor>, variables: Context) -> Result<Self> {
        let mut chain = Self::new(device, &variables);
        // right now descriptors need to be in the "correct" order, so that all blocks that depend
        // on something are encountered after the blocks they depend on.
        for (idx, descriptor) in descriptors.iter().enumerate() {
            let block: Rc<dyn ComputeBlock> = match descriptor {
                BlockDescriptor::Curve(desc) => Rc::new(CurveBlock::new(&chain, device, desc)),
                BlockDescriptor::Interval(desc) => Rc::new(IntervalBlock::new(&chain, device, desc)),
            };
            chain.insert(idx as u16, block)?;
        }

        return Ok(chain);
    }
}


