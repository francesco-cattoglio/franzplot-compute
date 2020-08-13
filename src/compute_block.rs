use crate::compute_chain;

use std::rc::*;
use std::any::Any;

pub trait ComputeBlock {
    fn get_pipeline(&self, encoder: &wgpu::CommandEncoder) -> &wgpu::ComputePipeline;
    fn get_output_buffer(&self) -> &wgpu::Buffer;
}

pub trait AnyCompute : ComputeBlock + Any {}

pub struct IntervalBlock {
    out_buffer: wgpu::Buffer,
    buffer_size: wgpu::BufferAddress,
}

impl IntervalBlock {
    pub fn new(compute_chain: &compute_chain::ComputeChain, descriptor: &IntervalBlockDescriptor) -> Self {
        let buffer_size = 16;
        let out_buffer = compute_chain.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_size,
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::STORAGE,
        });
        Self {
            out_buffer,
            buffer_size,
        }
    }
}

impl ComputeBlock for IntervalBlock {
    fn get_pipeline(&self, encoder: &wgpu::CommandEncoder) -> &wgpu::ComputePipeline {
        unimplemented!()
    }

    fn get_output_buffer(&self) -> &wgpu::Buffer {
        unimplemented!()
    }

}


pub struct IntervalBlockDescriptor {
    begin: f32,
    end: f32,
    name: String,
}

pub struct CurveBlockDescriptor {
        interval_input_idx: u16,
        x_function: String,
        y_function: String,
        z_function: String,
}

pub struct CurveBlock {
    out_buffer: wgpu::Buffer,
    buffer_size: wgpu::BufferAddress,
    interval_input: Rc<IntervalBlock>,
}

impl CurveBlock {
    pub fn new(compute_chain: &compute_chain::ComputeChain, descriptor: &CurveBlockDescriptor) -> Self {
        let interval_rc = compute_chain.blocks.get(&descriptor.interval_input_idx).expect("unable to find dependency");
        let interval_input = Rc::clone(&interval_rc).downcast::<IntervalBlock>().unwrap();
        let buffer_size = interval_input.buffer_size;
        let out_buffer = compute_chain.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_size,
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::STORAGE,
        });
        Self {
            out_buffer,
            buffer_size,
            interval_input,
        }
    }
}

impl ComputeBlock for CurveBlock {
    fn get_pipeline(&self, encoder: &wgpu::CommandEncoder) -> &wgpu::ComputePipeline {
        unimplemented!()
    }

    fn get_output_buffer(&self) -> &wgpu::Buffer {
        unimplemented!()
    }

}
