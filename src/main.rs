use maplit::btreemap;

mod device_manager;
mod compute_chain;
mod compute_block;

fn main() {
    use compute_block::*;
    let device_manager = device_manager::Manager::new();

    let curve_quality = 4;
    let first_descriptor = BlockDescriptor {
        id: "1".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "a".to_string(),
            end: "b".to_string(),
            quality: curve_quality,
            name: "k".to_string(),
        })
    };
    let second_descriptor = BlockDescriptor {
        id: "2".to_string(),
        data: DescriptorData::Curve(CurveBlockDescriptor {
            interval_input_id: "1".to_string(),
            x_function: "sin(k)".to_string(),
            y_function: "cos(k)".to_string(),
            z_function: "k".to_string(),
//            x_function: "a".to_string(),
//            y_function: "b".to_string(),
//            z_function: "a+b".to_string(),
        })
    };

    let all_variables = compute_chain::Context {
        globals: btreemap!{
            "a".to_string() => 0.0,
            "b".to_string() => 3.1415,
        },
    };

    let all_descriptors: Vec<BlockDescriptor> = vec![first_descriptor, second_descriptor].into();

    dbg!(&all_descriptors);
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, all_descriptors, all_variables).unwrap();

    chain.run_chain(&device_manager.device, &device_manager.queue);
    println!("Hello, world!");
{
    let staging_buffer = device_manager.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        mapped_at_creation: false,
        size: (std::mem::size_of::<f32>() * 4 * 64) as wgpu::BufferAddress,
        usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
    });

    let mut encoder =
        device_manager.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Compute Encoder this time"),
    });
    encoder.copy_buffer_to_buffer(
        chain.chain.get("2").unwrap().get_buffer(),
        0,
        &staging_buffer,
        0,
        (std::mem::size_of::<f32>() * 4 * 16*curve_quality as usize) as wgpu::BufferAddress,
    );

    let compute_queue = encoder.finish();
    device_manager.queue.submit(std::iter::once(compute_queue));

    let buffer_slice = staging_buffer.slice(..);
    // Gets the future representing when `staging_buffer` can be read from
    let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

    // Poll the device in a blocking manner so that our future resolves.
    // In an actual application, `device.poll(...)` should
    // be called in an event loop or on another thread.
    device_manager.device.poll(wgpu::Maintain::Wait);

    // Awaits until `buffer_future` can be read from
    use std::convert::TryInto;
    use futures::executor::block_on;
    let future_result = block_on(buffer_future);
    if let Ok(()) = future_result {
        // Gets contents of buffer
        let data = buffer_slice.get_mapped_range();
        // Since contents are got in bytes, this converts these bytes back to u32
        let result: Vec<f32> = data
            .chunks_exact(4)
            .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
            .skip(0)
            .step_by(1)
            .collect();

        // With the current interface, we have to make sure all mapped views are
        // dropped before we unmap the buffer.
        drop(data);
        staging_buffer.unmap(); // Unmaps buffer from memory
                                // If you are familiar with C++ these 2 lines can be thought of similarly to:
                                //   delete myPointer;
                                //   myPointer = NULL;
                                // It effectively frees the memory

        // Returns data from buffer
        dbg!(&result);
    } else {
        panic!("failed to run compute on gpu!")
    }

}
    let new_variables = compute_chain::Context {
        globals: btreemap!{
            "a".to_string() => 0.5*3.1415,
            "b".to_string() => 3.1415,
        },
    };

    println!("updated variables:");
    dbg!(&new_variables);
    chain.update_globals(&device_manager.device, &device_manager.queue, &new_variables);
    chain.run_chain(&device_manager.device, &device_manager.queue);
{
    let staging_buffer = device_manager.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        mapped_at_creation: false,
        size: (std::mem::size_of::<f32>() * 4 * 64) as wgpu::BufferAddress,
        usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
    });

    let mut encoder =
        device_manager.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Compute Encoder this time"),
    });
    encoder.copy_buffer_to_buffer(
        chain.chain.get("2").unwrap().get_buffer(),
        0,
        &staging_buffer,
        0,
        (std::mem::size_of::<f32>() * 4 * 16*curve_quality as usize) as wgpu::BufferAddress,
    );

    let compute_queue = encoder.finish();
    device_manager.queue.submit(std::iter::once(compute_queue));

    let buffer_slice = staging_buffer.slice(..);
    // Gets the future representing when `staging_buffer` can be read from
    let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

    // Poll the device in a blocking manner so that our future resolves.
    // In an actual application, `device.poll(...)` should
    // be called in an event loop or on another thread.
    device_manager.device.poll(wgpu::Maintain::Wait);

    use std::convert::TryInto;
    use futures::executor::block_on;
    let future_result = block_on(buffer_future);
    if let Ok(()) = future_result {
        // Gets contents of buffer
        let data = buffer_slice.get_mapped_range();
        // Since contents are got in bytes, this converts these bytes back to u32
        let result: Vec<f32> = data
            .chunks_exact(4)
            .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
            .skip(0)
            .step_by(1)
            .collect();

        // With the current interface, we have to make sure all mapped views are
        // dropped before we unmap the buffer.
        drop(data);
        staging_buffer.unmap(); // Unmaps buffer from memory
                                // If you are familiar with C++ these 2 lines can be thought of similarly to:
                                //   delete myPointer;
                                //   myPointer = NULL;
                                // It effectively frees the memory

        // Returns data from buffer
        dbg!(&result);
    } else {
        panic!("failed to run compute on gpu!")
    }
}
}
