use maplit::btreemap;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
};

mod camera;
mod texture;
mod rendering;
mod device_manager;
mod compute_chain;
mod compute_block;
mod shader_processing;
#[cfg(test)]
mod tests;

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

use compute_chain::Context;
use compute_block::*;
pub fn surface_chain_descriptors() -> (Context, Vec<BlockDescriptor>) {
    let all_variables = Context {
        globals: btreemap!{
            "a".to_string() => 0.0,
            "b".to_string() => 2.0,
            "t".to_string() => 0.0,
            "pi".to_string() => std::f32::consts::PI,
        },
    };

    let curve_quality = 1;
    let first_descriptor = BlockDescriptor {
        id: "1".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "-pi/2".to_string(),
            end: "pi/2".to_string(),
            quality: curve_quality,
            name: "u".to_string(),
        })
    };
    let second_descriptor = BlockDescriptor {
        id: "2".to_string(),
        data: DescriptorData::Interval(IntervalBlockDescriptor {
            begin: "0".to_string(),
            end: "0.8*pi".to_string(),
            quality: curve_quality,
            name: "v".to_string(),
        })
    };
    let surface_descriptor = BlockDescriptor {
        id: "3".to_string(),
        data: DescriptorData::Surface(SurfaceBlockDescriptor {
            interval_first_id: "1".to_string(),
            interval_second_id: "2".to_string(),
            x_function: "2*sin(u)".to_string(),
            y_function: "2*cos(u)*cos(v)".to_string(),
            z_function: "2*cos(u)*sin(v)".to_string(),
        })
    };

    let all_descriptors: Vec<BlockDescriptor> = vec![first_descriptor, second_descriptor, surface_descriptor];

    (all_variables, all_descriptors)

}


fn main() {
    let event_loop = EventLoop::new();
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title("test");
    #[cfg(windows_OFF)] // TODO check for news regarding this
    {
        use winit::platform::windows::WindowBuilderExtWindows;
        builder = builder.with_no_redirection_bitmap(true);
    }
    let window = builder.build(&event_loop).unwrap();

    let mut device_manager = device_manager::Manager::new(&window);

    let (all_variables, all_descriptors) = surface_chain_descriptors();

    dbg!(&all_descriptors);
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, all_descriptors, all_variables).unwrap();
    chain.run_chain(&device_manager.device, &device_manager.queue);
    let output_block = chain.chain.get("3").expect("could not find curve block");

    // let renderer = renderer::Renderer::new(&device_manager, out_buffer_slice);
    let renderer = rendering::SurfaceRenderer::new(&device_manager, output_block);
    renderer.model.update(&device_manager);

    let mut elapsed_time = std::time::Duration::from_secs(0);
    let mut old_instant = std::time::Instant::now();
    event_loop.run(move |event, _, control_flow| {
        let now = std::time::Instant::now();

        let frame_duration = now.duration_since(old_instant);
        if frame_duration.as_millis() > 0 {
            //println!("frame time: {} ms", frame_duration.as_millis());
            elapsed_time += frame_duration;
        }
        old_instant = now;
        match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::KeyboardInput { input, .. } => match input {
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        _ => {}
                    },
                    WindowEvent::Resized(_physical_size) => {
                    }
                    _ => {}
                }
        }
        Event::RedrawRequested(_) => {
            // update variables and do the actual rendering
        // now, update the variables and run the chain again
        let new_variables = compute_chain::Context {
            globals: btreemap!{
                "a".to_string() => 0.0,
                "b".to_string() => 2.0,
                "t".to_string() => elapsed_time.as_secs_f32(),
                "pi".to_string() => std::f32::consts::PI,
            },
        };
        chain.update_globals(&device_manager.queue, &new_variables);
        chain.run_chain(&device_manager.device, &device_manager.queue);
            let mut frame = device_manager.swap_chain.get_current_frame()
                .expect("could not get next frame");

            let surface_block = chain.chain.get("3").expect("could not find curve block");
            let surface_buffer = surface_block.get_buffer();
//            renderer.model.update_vertex_buffer(&device_manager, surface_buffer);
            renderer.render(&device_manager, &mut frame);
        }
        Event::MainEventsCleared => {
            // RedrawRequested will only trigger once, unless we manually
            // request it.
            window.request_redraw();
        }
        _ => {}
    }
    });
}
