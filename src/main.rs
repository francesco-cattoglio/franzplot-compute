use maplit::btreemap;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
};
use serde::{Deserialize, Serialize};

mod util;
mod camera;
mod texture;
mod rendering;
mod device_manager;
mod compute_chain;
mod compute_block;
mod shader_processing;
#[cfg(test)]
mod tests;

#[derive(Debug, Deserialize, Serialize)]
struct SceneDescriptor {
    context: compute_chain::Context,
    descriptors: Vec<BlockDescriptor>,
}

use compute_block::*;
use getopts::Options;
use std::env;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

use std::io::prelude::*;
fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("i", "", "set input file name", "NAME");
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let input_file = match matches.opt_str("i") {
        Some(input) => input,
        None => {
            print_usage(&program, opts);
            return;
        }
    };

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

    let mut json_contents = String::new();
    let mut file = std::fs::File::open(&input_file).unwrap();
    file.read_to_string(&mut json_contents).unwrap();
    let json_scene: SceneDescriptor = serde_json::from_str(&json_contents).unwrap();
    //dbg!(&all_descriptors);
    let mut chain = compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, &json_scene.descriptors, &json_scene.context).unwrap();
    chain.run_chain(&device_manager.device, &device_manager.queue);

    //let dbg_buff = chain.chain.get("").expect("wrong block name for dbg printout").output_renderer.get_buffer();
    //let dbg_vect = copy_buffer_as_f32(out_buff, &device_manager.device);
    //println!("debugged buffer contains {:?}", dbg_vect);

    // let renderer = renderer::Renderer::new(&device_manager, out_buffer_slice);
    let mut renderer = rendering::SurfaceRenderer::new(&device_manager);
    renderer.update_renderables(&device_manager, &chain);

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
