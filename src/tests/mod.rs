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
    let (window, device_manager) = setup_test();
    let (all_variables, all_descriptors) = interval_matrix_descriptors();

    dbg!(&all_descriptors);
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, all_descriptors, all_variables).unwrap();

    chain.run_chain(&device_manager.device, &device_manager.queue);
    println!("Hello, world!");
    let output_block = chain.chain.get("2").expect("could not find curve block");
    let out_data = copy_buffer_as_f32(output_block.get_buffer(), &device_manager.device);
    dbg!(out_data);
}

fn setup_test() -> (winit::window::Window, device_manager::Manager) {
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

    (window, device_manager)
}

#[test]
fn simple_point_transform () {
    let (_window, device_manager) = setup_test();

    // define descriptors for pointinterval, curve, simple matrix and transform
    let all_variables = Context {
        globals: btreemap!{
            "a".to_string() => 0.33333,
            "pi".to_string() => 3.1415,
        },
    };

    let point_desc = BlockDescriptor {
        id: "1".to_string(),
        data: DescriptorData::Point(PointBlockDescriptor {
            fx: "a".to_string(),
            fy: "0".to_string(),
            fz: "1".to_string(),
        })
    };
    let matrix_desc = BlockDescriptor {
        id: "2".to_string(),
        data: DescriptorData::Matrix(MatrixBlockDescriptor {
            interval_id: None,
        })
    };
    let transform_desc = BlockDescriptor {
        id: "3".to_string(),
        data: DescriptorData::Transform(TransformBlockDescriptor {
            geometry_id: "1".to_string(),
            matrix_id: "2".to_string(),
        })
    };

    let all_descriptors: Vec<BlockDescriptor> = vec![point_desc, matrix_desc, transform_desc].into();

    dbg!(&all_descriptors);
    println!("Running the chain");
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, all_descriptors, all_variables).unwrap();
    chain.run_chain(&device_manager.device, &device_manager.queue);
    let point_block = chain.chain.get("1").expect("could not find point block");
    let point_data = copy_buffer_as_f32(point_block.get_buffer(), &device_manager.device);
    dbg!(point_data);
    let transformed_block = chain.chain.get("3").expect("could not find transformed block");
    let transformed_data = copy_buffer_as_f32(transformed_block.get_buffer(), &device_manager.device);
    dbg!(transformed_data);
}

#[test]
fn interval_point_transform () {
    let (_window, device_manager) = setup_test();

    // define descriptors for pointinterval, curve, simple matrix and transform
    let all_variables = Context {
        globals: btreemap!{
            "a".to_string() => 0.123456,
            "pi".to_string() => 3.1415,
        },
    };

    let point_desc = BlockDescriptor {
        id: "1".to_string(),
        data: DescriptorData::Point(PointBlockDescriptor {
            fx: "a".to_string(),
            fy: "0".to_string(),
            fz: "-1".to_string(),
        })
    };
    let interval_desc = BlockDescriptor {
        id: "@s".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "0".to_string(),
            end: "2".to_string(),
            quality: 2,
            name: "s".to_string(),
        })
    };
    let matrix_desc = BlockDescriptor {
        id: "2".to_string(),
        data: DescriptorData::Matrix(MatrixBlockDescriptor {
            interval_id: Some("@s".to_string()),
        })
    };
    let transform_desc = BlockDescriptor {
        id: "3".to_string(),
        data: DescriptorData::Transform(TransformBlockDescriptor {
            geometry_id: "1".to_string(),
            matrix_id: "2".to_string(),
        })
    };

    let all_descriptors: Vec<BlockDescriptor> = vec![point_desc, interval_desc, matrix_desc, transform_desc].into();

    dbg!(&all_descriptors);
    println!("Running the chain");
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, all_descriptors, all_variables).unwrap();
    chain.run_chain(&device_manager.device, &device_manager.queue);
    let point_block = chain.chain.get("1").expect("could not find point block");
    let point_data = copy_buffer_as_f32(point_block.get_buffer(), &device_manager.device);
    dbg!(point_data);
    let transformed_block = chain.chain.get("3").expect("could not find transformed block");
    let transformed_data = copy_buffer_as_f32(transformed_block.get_buffer(), &device_manager.device);
    dbg!(transformed_data);
}

#[test]
fn simple_curve_transform () {
    let (_window, device_manager) = setup_test();

    // define descriptors for interval, curve, simple matrix and transform

    let interval_desc = BlockDescriptor {
        id: "1".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "0".to_string(),
            end: "1".to_string(),
            quality: 2,
            name: "s".to_string(),
        })
    };
    let curve_desc = BlockDescriptor {
        id: "2".to_string(),
        data: DescriptorData::Curve(CurveBlockDescriptor {
            interval_input_id: "1".to_string(),
            x_function: "s".to_string(),
            y_function: "0.0".to_string(),
            z_function: "0.0".to_string(),
        })
    };
    let matrix_desc = BlockDescriptor {
        id: "3".to_string(),
        data: DescriptorData::Matrix(MatrixBlockDescriptor {
            interval_id: None,
        })
    };
    let transform_desc = BlockDescriptor {
        id: "4".to_string(),
        data: DescriptorData::Transform(TransformBlockDescriptor {
            geometry_id: "2".to_string(),
            matrix_id: "3".to_string(),
        })
    };

    let all_variables = Context {
        globals: btreemap!{
            "pi".to_string() => 3.1415,
        },
    };
    let all_descriptors: Vec<BlockDescriptor> = vec![interval_desc, curve_desc, matrix_desc, transform_desc].into();

    dbg!(&all_descriptors);
    println!("Running the chain");
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, all_descriptors, all_variables).unwrap();
    chain.run_chain(&device_manager.device, &device_manager.queue);
    let curve_block = chain.chain.get("2").expect("could not find curve block");
    let curve_data = copy_buffer_as_f32(curve_block.get_buffer(), &device_manager.device);
    dbg!(curve_data);
    let transformed_block = chain.chain.get("4").expect("could not find curve block");
    let transformed_data = copy_buffer_as_f32(transformed_block.get_buffer(), &device_manager.device);
    dbg!(transformed_data);
}

#[test]
fn same_parameter_curve_transform () {
    let (_window, device_manager) = setup_test();

    // define descriptors for interval, curve, simple matrix and transform

    let interval_desc = BlockDescriptor {
        id: "1".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "0".to_string(),
            end: "1".to_string(),
            quality: 2,
            name: "s".to_string(),
        })
    };
    let curve_desc = BlockDescriptor {
        id: "2".to_string(),
        data: DescriptorData::Curve(CurveBlockDescriptor {
            interval_input_id: "1".to_string(),
            x_function: "s".to_string(),
            y_function: "0.0".to_string(),
            z_function: "0.0".to_string(),
        })
    };
    let matrix_desc = BlockDescriptor {
        id: "3".to_string(),
        data: DescriptorData::Matrix(MatrixBlockDescriptor {
            interval_id: Some("1".to_string()),
        })
    };
    let transform_desc = BlockDescriptor {
        id: "4".to_string(),
        data: DescriptorData::Transform(TransformBlockDescriptor {
            geometry_id: "2".to_string(),
            matrix_id: "3".to_string(),
        })
    };

    let all_variables = Context {
        globals: btreemap!{
            "pi".to_string() => 3.1415,
        },
    };
    let all_descriptors: Vec<BlockDescriptor> = vec![interval_desc, curve_desc, matrix_desc, transform_desc].into();

    dbg!(&all_descriptors);
    println!("Running the chain");
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, all_descriptors, all_variables).unwrap();
    chain.run_chain(&device_manager.device, &device_manager.queue);
    let curve_block = chain.chain.get("2").expect("could not find curve block");
    let curve_data = copy_buffer_as_f32(curve_block.get_buffer(), &device_manager.device);
    dbg!(curve_data);
    let transformed_block = chain.chain.get("4").expect("could not find curve block");
    let transformed_data = copy_buffer_as_f32(transformed_block.get_buffer(), &device_manager.device);
    dbg!(transformed_data);
}

#[test]
fn transform_1d_2up () {
    let (_window, device_manager) = setup_test();

    // define descriptors for interval, curve, simple matrix and transform

    let interval_s_desc = BlockDescriptor {
        id: "@s".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "0".to_string(),
            end: "1".to_string(),
            quality: 1,
            name: "s".to_string(),
        })
    };
    let interval_t_desc = BlockDescriptor {
        id: "@t".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "0".to_string(),
            end: "1".to_string(),
            quality: 1,
            name: "t".to_string(),
        })
    };
    let curve_desc = BlockDescriptor {
        id: "2".to_string(),
        data: DescriptorData::Curve(CurveBlockDescriptor {
            interval_input_id: "@s".to_string(),
            x_function: "s".to_string(),
            y_function: "0.0".to_string(),
            z_function: "0.0".to_string(),
        })
    };
    let matrix_desc = BlockDescriptor {
        id: "3".to_string(),
        data: DescriptorData::Matrix(MatrixBlockDescriptor {
            interval_id: Some("@t".to_string()),
        })
    };
    let transform_desc = BlockDescriptor {
        id: "4".to_string(),
        data: DescriptorData::Transform(TransformBlockDescriptor {
            geometry_id: "2".to_string(),
            matrix_id: "3".to_string(),
        })
    };

    let all_variables = Context {
        globals: btreemap!{
            "pi".to_string() => 3.1415,
        },
    };
    let all_descriptors: Vec<BlockDescriptor> = vec![interval_s_desc, interval_t_desc, curve_desc, matrix_desc, transform_desc].into();

    dbg!(&all_descriptors);
    println!("Running the chain");
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, all_descriptors, all_variables).unwrap();
    chain.run_chain(&device_manager.device, &device_manager.queue);
    let curve_block = chain.chain.get("2").expect("could not find curve block");
    let curve_data = copy_buffer_as_f32(curve_block.get_buffer(), &device_manager.device);
    dbg!(curve_data);
    let transformed_block = chain.chain.get("4").expect("could not find curve block");
    let transformed_data = copy_buffer_as_f32(transformed_block.get_buffer(), &device_manager.device);
    dbg!(transformed_data);
}

#[test]
fn simple_surface_transform () {
    let (_window, device_manager) = setup_test();

    // define descriptors for interval, curve, simple matrix and transform

    let interval_s_desc = BlockDescriptor {
        id: "@s".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "0".to_string(),
            end: "1".to_string(),
            quality: 1,
            name: "s".to_string(),
        })
    };
    let interval_t_desc = BlockDescriptor {
        id: "@t".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "0".to_string(),
            end: "1".to_string(),
            quality: 1,
            name: "t".to_string(),
        })
    };
    let surf_desc = BlockDescriptor {
        id: "2".to_string(),
        data: DescriptorData::Surface(SurfaceBlockDescriptor {
            interval_first_id: "@s".to_string(),
            interval_second_id: "@t".to_string(),
            x_function: "s".to_string(),
            y_function: "t".to_string(),
            z_function: "0.0".to_string(),
        })
    };
    let matrix_desc = BlockDescriptor {
        id: "3".to_string(),
        data: DescriptorData::Matrix(MatrixBlockDescriptor {
            interval_id: None,
        })
    };
    let transform_desc = BlockDescriptor {
        id: "4".to_string(),
        data: DescriptorData::Transform(TransformBlockDescriptor {
            geometry_id: "2".to_string(),
            matrix_id: "3".to_string(),
        })
    };

    let all_variables = Context {
        globals: btreemap!{
            "pi".to_string() => 3.1415,
        },
    };
    let all_descriptors: Vec<BlockDescriptor> = vec![interval_s_desc, interval_t_desc, surf_desc, matrix_desc, transform_desc].into();

    dbg!(&all_descriptors);
    println!("Running the chain");
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, all_descriptors, all_variables).unwrap();
    chain.run_chain(&device_manager.device, &device_manager.queue);
    let surf_block = chain.chain.get("2").expect("could not find curve block");
    let surf_data = copy_buffer_as_f32(surf_block.get_buffer(), &device_manager.device);
    dbg!(surf_data);
    let transformed_block = chain.chain.get("4").expect("could not find curve block");
    let transformed_data = copy_buffer_as_f32(transformed_block.get_buffer(), &device_manager.device);
    dbg!(transformed_data);
}

#[test]
fn same_param1_surface_transform () {
    let (_window, device_manager) = setup_test();

    // define descriptors for interval, curve, simple matrix and transform

    let interval_s_desc = BlockDescriptor {
        id: "@s".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "0".to_string(),
            end: "1".to_string(),
            quality: 1,
            name: "s".to_string(),
        })
    };
    let interval_t_desc = BlockDescriptor {
        id: "@t".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "0".to_string(),
            end: "1".to_string(),
            quality: 1,
            name: "t".to_string(),
        })
    };
    let surf_desc = BlockDescriptor {
        id: "2".to_string(),
        data: DescriptorData::Surface(SurfaceBlockDescriptor {
            interval_first_id: "@s".to_string(),
            interval_second_id: "@t".to_string(),
            x_function: "s".to_string(),
            y_function: "t".to_string(),
            z_function: "0.0".to_string(),
        })
    };
    let matrix_desc = BlockDescriptor {
        id: "3".to_string(),
        data: DescriptorData::Matrix(MatrixBlockDescriptor {
            interval_id: Some("@s".to_string()),
        })
    };
    let transform_desc = BlockDescriptor {
        id: "4".to_string(),
        data: DescriptorData::Transform(TransformBlockDescriptor {
            geometry_id: "2".to_string(),
            matrix_id: "3".to_string(),
        })
    };

    let all_variables = Context {
        globals: btreemap!{
            "pi".to_string() => 3.1415,
        },
    };
    let all_descriptors: Vec<BlockDescriptor> = vec![interval_s_desc, interval_t_desc, surf_desc, matrix_desc, transform_desc].into();

    dbg!(&all_descriptors);
    println!("Running the chain");
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, all_descriptors, all_variables).unwrap();
    chain.run_chain(&device_manager.device, &device_manager.queue);
    let surf_block = chain.chain.get("2").expect("could not find curve block");
    let surf_data = copy_buffer_as_f32(surf_block.get_buffer(), &device_manager.device);
    dbg!(surf_data);
    let transformed_block = chain.chain.get("4").expect("could not find curve block");
    let transformed_data = copy_buffer_as_f32(transformed_block.get_buffer(), &device_manager.device);
    dbg!(transformed_data);
}

#[test]
fn same_param2_surface_transform () {
    let (_window, device_manager) = setup_test();

    // define descriptors for interval, curve, simple matrix and transform

    let interval_s_desc = BlockDescriptor {
        id: "@s".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "0".to_string(),
            end: "1".to_string(),
            quality: 1,
            name: "s".to_string(),
        })
    };
    let interval_t_desc = BlockDescriptor {
        id: "@t".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "0".to_string(),
            end: "1".to_string(),
            quality: 1,
            name: "t".to_string(),
        })
    };
    let surf_desc = BlockDescriptor {
        id: "2".to_string(),
        data: DescriptorData::Surface(SurfaceBlockDescriptor {
            interval_first_id: "@s".to_string(),
            interval_second_id: "@t".to_string(),
            x_function: "s".to_string(),
            y_function: "t".to_string(),
            z_function: "0.0".to_string(),
        })
    };
    let matrix_desc = BlockDescriptor {
        id: "3".to_string(),
        data: DescriptorData::Matrix(MatrixBlockDescriptor {
            interval_id: Some("@t".to_string()),
        })
    };
    let transform_desc = BlockDescriptor {
        id: "4".to_string(),
        data: DescriptorData::Transform(TransformBlockDescriptor {
            geometry_id: "2".to_string(),
            matrix_id: "3".to_string(),
        })
    };

    let all_variables = Context {
        globals: btreemap!{
            "pi".to_string() => 3.1415,
        },
    };
    let all_descriptors: Vec<BlockDescriptor> = vec![interval_s_desc, interval_t_desc, surf_desc, matrix_desc, transform_desc].into();

    dbg!(&all_descriptors);
    println!("Running the chain");
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, all_descriptors, all_variables).unwrap();
    chain.run_chain(&device_manager.device, &device_manager.queue);
    let surf_block = chain.chain.get("2").expect("could not find curve block");
    let surf_data = copy_buffer_as_f32(surf_block.get_buffer(), &device_manager.device);
    dbg!(surf_data);
    let transformed_block = chain.chain.get("4").expect("could not find curve block");
    let transformed_data = copy_buffer_as_f32(transformed_block.get_buffer(), &device_manager.device);
    dbg!(transformed_data);
}
