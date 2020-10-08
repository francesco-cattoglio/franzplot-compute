use maplit::btreemap;
use crate::compute_block::*;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use wgpu::util::DeviceExt;

pub struct ComputeChain {
    compute_blocks: Vec<ComputeBlock>,
    blocks_map: BTreeMap<String, usize>,
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

const MAX_NUM_GLOBALS: usize = 32;

impl<'a> ComputeChain {
    pub fn new(device: &wgpu::Device) -> Self {

        let globals_buffer_size = (MAX_NUM_GLOBALS * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor{
            label: Some("globals buffer"),
            mapped_at_creation: false,
            size: globals_buffer_size,
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
                label: Some("Globals uniform layout")
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

        Self {
            globals_bind_layout,
            globals_bind_group,
            globals_buffer,
            globals_buffer_size,
            compute_blocks: Vec::new(),
            blocks_map: BTreeMap::new(),
            global_vars: BTreeSet::new(),
            shader_header: String::new(),
        }
    }

    pub fn set_scene(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, context: &Context, descriptors: &Vec<BlockDescriptor>) -> Result<()> {
        assert!(context.globals.len() <= MAX_NUM_GLOBALS);
        // cleanup previously stored context, shader_header and blocks
        self.compute_blocks.clear();
        self.shader_header.clear();
        self.global_vars.clear();

        // now re-process the context and the descriptors
        // update the stored names of the globals
        self.global_vars = context.globals.keys().cloned().collect();
        // store a copy of the mapped values in our buffer
        let values: Vec<f32> = context.globals.values().copied().collect();
        queue.write_buffer(&self.globals_buffer, 0, bytemuck::cast_slice(&values));
        // write the shader header that will be used in the creation of the compute pipeline shaders
        self.shader_header.push_str("layout(set = 1, binding = 0) uniform Uniforms {\n");
        for var_name in self.global_vars.iter() {
            self.shader_header.push_str(format!("\tfloat {};\n", var_name).as_str());
        }
        self.shader_header.push_str("};\n");
        //println!("debug info for shader header: {}", &shader_header);
        // now turn the block descriptors into block and insert them into the map
        for descriptor in descriptors.iter() {
            let block = descriptor.data.to_block(&self, device);
            self.insert(descriptor.id.clone(), block)?;
        }

        Ok(())
    }

    pub fn run_chain(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder this time"),
        });
        for block in self.compute_blocks.iter() {
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
        if self.blocks_map.contains_key(&id) {
            Err(anyhow!("Tried to insert two blocks that had the same id"))
        } else {
            // append the block to the vector and map its index in the blocks_map
            let idx = self.compute_blocks.len(); // the index at which we store is the size of the vec before pushing
            self.blocks_map.insert(id, idx);
            self.compute_blocks.push(block);
            Ok(())
        }
    }

    pub fn create_from_descriptors(device: &wgpu::Device, queue: &wgpu::Queue, context: &Context, descriptors: &Vec<BlockDescriptor>) -> Result<Self> {
        let mut chain = Self::new(device);
        chain.set_scene(device, queue, context, descriptors)?;

        Ok(chain)
    }

    pub fn get_block(&'a self, id: &String) -> Option<&'a ComputeBlock> {
        let idx = self.blocks_map.get(id)?;
        self.compute_blocks.get(*idx)
    }

    pub fn blocks_iterator(&'a self) -> std::slice::Iter<ComputeBlock> {
        self.compute_blocks.iter()
    }

}


