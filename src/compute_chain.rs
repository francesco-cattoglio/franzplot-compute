use maplit::btreemap;
use crate::compute_block::*;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use wgpu::util::DeviceExt;

pub struct ComputeChain {
    pub chain: BTreeMap<String, ComputeBlock>,
    globals_buffer_size: wgpu::BufferAddress,
    globals_buffer: wgpu::Buffer,
    pub globals_bind_layout: wgpu::BindGroupLayout,
    pub globals_bind_group: wgpu::BindGroup,
    pub shader_header: String,
    pub global_vars: BTreeSet<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Context {
    pub globals: BTreeMap<String, f32>,
}

impl std::default::Default for Context {
    fn default() -> Self {
        Self {
            globals: btreemap!{
                "t".to_string() => 0.0,
                "pi".to_string() => std::f32::consts::PI,
            },
        }
    }
}

impl ComputeChain {
    pub fn new(device: &wgpu::Device, context: &Context) -> Self {
        let globals = &context.globals;
        let global_vars: BTreeSet<String> = globals.keys().cloned().collect();
        let chain = BTreeMap::<String, ComputeBlock>::new();

        let values: Vec<f32> = globals.values().copied().collect();
        let globals_buffer_size = (values.len() * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let globals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&values),
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM
        });
        let globals_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        count: None,
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::UniformBuffer {
                            min_binding_size: None,
                            dynamic: false,
                        }
                    },
                ],
                label: Some("Variables uniform layout")
            });
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &globals_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer (
                        globals_buffer.slice(..),
                    ),
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
        //println!("debug info for shader header: {}", &shader_header);

        Self {
            chain,
            shader_header,
            global_vars,
            globals_bind_layout,
            globals_bind_group,
            globals_buffer,
            globals_buffer_size,
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
        queue.submit(std::iter::once(compute_queue));
    }

    pub fn update_globals(&mut self, queue: &wgpu::Queue, context: &Context) {
        // need to check if the global_vars contains exactly the same global
        // names as the context we passed to this function.
        assert!(self.global_vars.iter().eq(context.globals.keys()));
        // if this is true, then we need to move data into the globals buffer
        let values: Vec<f32> = context.globals.values().copied().collect();
        queue.write_buffer(&self.globals_buffer, 0, bytemuck::cast_slice(&values));
    }

    fn insert(&mut self, id: String, block: ComputeBlock) -> Result<()> {
        if self.chain.contains_key(&id) {
            Err(anyhow!("Tried to insert two blocks that had the same id"))
        } else {
            self.chain.insert(id, block);
            Ok(())
        }
    }

    pub fn create_from_descriptors(device: &wgpu::Device, descriptors: &Vec<BlockDescriptor>, globals: &Context) -> Result<Self> {
        let mut chain = Self::new(device, &globals);
        // right now descriptors need to be in the "correct" order, so that all blocks that depend
        // on something are encountered after the blocks they depend on.
        for descriptor in descriptors.iter() {
            let block = descriptor.data.to_block(&chain, &device);
            chain.insert(descriptor.id.clone(), block)?;
        }

        Ok(chain)
    }
}


