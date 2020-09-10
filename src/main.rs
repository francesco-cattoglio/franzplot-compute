use winit::dpi::PhysicalSize;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

mod device_manager;
mod compute_chain;
mod compute_block;
mod shader_processing;
mod testing_room;

// maps a buffer, waits for it to be available, and copies its contents into a new Vec<f32>
fn copy_buffer_as_f32(buffer: &wgpu::Buffer, device: &wgpu::Device) -> Vec<f32> {
    use futures::executor::block_on;
    let future_result = buffer.slice(..).map_async(wgpu::MapMode::Read);
    device.poll(wgpu::Maintain::Wait);
    block_on(future_result).unwrap();
    let mapped_buffer = buffer.slice(..).get_mapped_range();
    let data: &[u8] = &mapped_buffer;
    use std::convert::TryInto;
    // Since contents are got in bytes, this converts these bytes back to f32
    let result: Vec<f32> = data
        .chunks_exact(4)
        .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
        .skip(0)
        .step_by(1)
        .collect();
    // With the current interface, we have to make sure all mapped views are
    // dropped before we unmap the buffer.
    drop(mapped_buffer);
    buffer.unmap();

    result
}

fn main() {
    let device_manager = device_manager::Manager::new();

    let (all_variables, all_descriptors) = testing_room::interval_curve_test();

    dbg!(&all_descriptors);
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, all_descriptors, all_variables).unwrap();

    chain.run_chain(&device_manager.device, &device_manager.queue);
    println!("Hello, world!");
    let output_block = chain.chain.get("2").expect("could not find curve block");
    let out_data = copy_buffer_as_f32(output_block.get_buffer(), &device_manager.device);
    dbg!(out_data);
    use maplit::btreemap;
    let new_variables = compute_chain::Context {
        globals: btreemap!{
            "a".to_string() => 0.5*3.1415,
            "b".to_string() => 3.1415,
        },
    };

    println!("updated variables:");
    dbg!(&new_variables);
    chain.update_globals(&device_manager.queue, &new_variables);
    chain.run_chain(&device_manager.device, &device_manager.queue);
    let output_block = chain.chain.get("2").expect("could not find curve block");
    let out_data = copy_buffer_as_f32(output_block.get_buffer(), &device_manager.device);
    dbg!(out_data);
}
