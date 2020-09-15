use maplit::btreemap;

use crate::compute_block::*;
use crate::compute_chain::*;

pub fn interval_curve_test() -> (Context, Vec<BlockDescriptor>) {
    let all_variables = Context {
        globals: btreemap!{
            "a".to_string() => 0.0,
            "b".to_string() => 3.1415,
        },
    };

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

    let all_descriptors: Vec<BlockDescriptor> = vec![first_descriptor, second_descriptor].into();

    (all_variables, all_descriptors)
}

pub fn interval_surface_test() -> (Context, Vec<BlockDescriptor>) {
    let all_variables = Context {
        globals: btreemap!{
            "a".to_string() => 0.0,
            "b".to_string() => 1.0,
        },
    };

    let curve_quality = 1;
    let first_descriptor = BlockDescriptor {
        id: "1".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "a".to_string(),
            end: "b".to_string(),
            quality: curve_quality,
            name: "u".to_string(),
        })
    };
    let second_descriptor = BlockDescriptor {
        id: "2".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "a".to_string(),
            end: "b".to_string(),
            quality: curve_quality,
            name: "v".to_string(),
        })
    };
    let surface_descriptor = BlockDescriptor {
        id: "3".to_string(),
        data: DescriptorData::Surface(SurfaceBlockDescriptor {
            interval_first_id: "1".to_string(),
            interval_second_id: "2".to_string(),
            x_function: "u".to_string(),
            y_function: "0.25*sin(v*2*3.1451)".to_string(),
            z_function: "v".to_string(),
//            x_function: "a".to_string(),
//            y_function: "b".to_string(),
//            z_function: "a+b".to_string(),
        })
    };

    let all_descriptors: Vec<BlockDescriptor> = vec![first_descriptor, second_descriptor, surface_descriptor].into();

    (all_variables, all_descriptors)

}

use crate::device_manager;
use crate::compute_chain;
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

#[test]
fn test_curve_compute() {
    let event_loop = winit::event_loop::EventLoop::new();
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title("test");
    #[cfg(windows_OFF)] // TODO check for news regarding this
    {
        use winit::platform::windows::WindowBuilderExtWindows;
        builder = builder.with_no_redirection_bitmap(true);
    }
    let window = builder.build(&event_loop).unwrap();

    let mut device_manager = device_manager::Manager::new(&window);

    let (all_variables, all_descriptors) = interval_curve_test();

    dbg!(&all_descriptors);
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, all_descriptors, all_variables).unwrap();

    chain.run_chain(&device_manager.device, &device_manager.queue);
    println!("Hello, world!");
    let output_block = chain.chain.get("2").expect("could not find curve block");
    let out_data = copy_buffer_as_f32(output_block.get_buffer(), &device_manager.device);
    dbg!(out_data);

    // now, update the variables and run the chain again
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

pub fn simple_matrix_descriptors() -> (Context, Vec<BlockDescriptor>) {
    let all_variables = Context {
        globals: btreemap!{
            "a".to_string() => 0.0,
            "b".to_string() => 1.0,
        },
    };

    let curve_quality = 1;
    let first_descriptor = BlockDescriptor {
        id: "1".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "a".to_string(),
            end: "b".to_string(),
            quality: curve_quality,
            name: "u".to_string(),
        })
    };
    let second_descriptor = BlockDescriptor {
        id: "2".to_string(),
        data: DescriptorData::Matrix(MatrixBlockDescriptor {
            interval_id: None,
        })
    };

    let all_descriptors: Vec<BlockDescriptor> = vec![first_descriptor, second_descriptor].into();

    (all_variables, all_descriptors)
}

pub fn interval_matrix_descriptors() -> (Context, Vec<BlockDescriptor>) {
    let all_variables = Context {
        globals: btreemap!{
            "a".to_string() => 0.0,
            "b".to_string() => 1.0,
        },
    };

    let curve_quality = 1;
    let first_descriptor = BlockDescriptor {
        id: "1".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "a".to_string(),
            end: "b".to_string(),
            quality: curve_quality,
            name: "u".to_string(),
        })
    };
    let second_descriptor = BlockDescriptor {
        id: "2".to_string(),
        data: DescriptorData::Matrix(MatrixBlockDescriptor {
            interval_id: Some("1".to_string()),
        })
    };

    let all_descriptors: Vec<BlockDescriptor> = vec![first_descriptor, second_descriptor].into();

    (all_variables, all_descriptors)
}

#[test]
fn test_simple_matrix() {
    let event_loop = winit::event_loop::EventLoop::new();
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title("test");
    #[cfg(windows_OFF)] // TODO check for news regarding this
    {
        use winit::platform::windows::WindowBuilderExtWindows;
        builder = builder.with_no_redirection_bitmap(true);
    }
    let window = builder.build(&event_loop).unwrap();

    println!("abebe");
    let device_manager = device_manager::Manager::new(&window);

    let (all_variables, all_descriptors) = simple_matrix_descriptors();

    dbg!(&all_descriptors);
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, all_descriptors, all_variables).unwrap();

    chain.run_chain(&device_manager.device, &device_manager.queue);
    println!("Hello, world!");
    let output_block = chain.chain.get("2").expect("could not find curve block");
    let out_data = copy_buffer_as_f32(output_block.get_buffer(), &device_manager.device);
    dbg!(out_data);
}

#[test]
fn test_interval_matrix() {
    let event_loop = winit::event_loop::EventLoop::new();
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title("test");
    #[cfg(windows_OFF)] // TODO check for news regarding this
    {
        use winit::platform::windows::WindowBuilderExtWindows;
        builder = builder.with_no_redirection_bitmap(true);
    }
    let window = builder.build(&event_loop).unwrap();

    let device_manager = device_manager::Manager::new(&window);

    let (all_variables, all_descriptors) = interval_matrix_descriptors();

    dbg!(&all_descriptors);
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, all_descriptors, all_variables).unwrap();

    chain.run_chain(&device_manager.device, &device_manager.queue);
    println!("Hello, world!");
    let output_block = chain.chain.get("2").expect("could not find curve block");
    let out_data = copy_buffer_as_f32(output_block.get_buffer(), &device_manager.device);
    dbg!(out_data);
}
