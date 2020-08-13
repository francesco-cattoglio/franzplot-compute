mod device_manager;
mod compute_chain;
mod compute_block;

fn main() {
    let device_manager = device_manager::DeviceManager::new();

    let first_descriptor = compute_block::IntervalBlockDescriptor {
        begin: 0.0,
        end: 3.1415,
        quality: 4,
        name: "u".to_string(),
    };
    let second_descriptor = compute_block::CurveBlockDescriptor {
        interval_input_idx: 0,
        x_function: "sin(u)".to_string(),
        y_function: "cos(u)".to_string(),
        z_function: "0.25".to_string(),
    };

    use compute_chain::BlockDescriptor;
    let all_descriptors: Vec<BlockDescriptor> =
        vec![BlockDescriptor::Interval(first_descriptor), BlockDescriptor::Curve(second_descriptor)].into();

    dbg!(&all_descriptors);
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, all_descriptors).unwrap();

    chain.run_chain(&device_manager.device, &device_manager.queue);
    println!("Hello, world!");

    let staging_buffer = device_manager.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (std::mem::size_of::<f32>() * 4 * 64) as wgpu::BufferAddress,
        usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
    });

    let mut encoder =
        device_manager.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Compute Encoder this time"),
    });
    encoder.copy_buffer_to_buffer(
        chain.blocks.get(&1).unwrap().get_buffer(),
        0,
        &staging_buffer,
        0,
        (std::mem::size_of::<f32>() * 4 * 64) as wgpu::BufferAddress,
    );

    let compute_queue = encoder.finish();
    device_manager.queue.submit(&[compute_queue]);

    let buffer_future = staging_buffer.map_read(0,
            (std::mem::size_of::<f32>() * 4 * 64) as wgpu::BufferAddress);

    // Poll the device in a blocking manner so that our future resolves.
    // In an actual application, `device.poll(...)` should
    // be called in an event loop or on another thread.
    device_manager.device.poll(wgpu::Maintain::Wait);

    use std::convert::TryInto;
    use futures::executor::block_on;
    let future_result = block_on(buffer_future);
    if let Ok(ok_read_mapping) = future_result {
        let slice = ok_read_mapping.as_slice();
        let result: Vec<f32> = slice
            .chunks_exact(4)
            .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
            .collect();

        // With the current interface, we have to make sure all mapped views are
        // dropped before we unmap the buffer.
        staging_buffer.unmap();

        dbg!(result);
    }

}
