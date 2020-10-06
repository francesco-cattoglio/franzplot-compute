use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
};
use imgui::*;
use imgui_wgpu::Renderer;
use imgui_winit_support;
use serde::{Deserialize, Serialize};

mod util;
mod camera;
mod texture;
mod rendering;
mod device_manager;
mod compute_chain;
mod compute_block;
mod shader_processing;
mod cpp_gui;
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

    let input_file = matches.opt_str("i");

    //wgpu_subscriber::initialize_default_subscriber(None);

    let event_loop = EventLoop::new();
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder
        .with_title("test")
        .with_inner_size(winit::dpi::PhysicalSize::new(1280, 800));
    #[cfg(windows_OFF)] // TODO check for news regarding this
    {
        use winit::platform::windows::WindowBuilderExtWindows;
        builder = builder.with_no_redirection_bitmap(true);
    }
    let window = builder.build(&event_loop).unwrap();

    let mut hidpi_factor = window.scale_factor();

    let mut device_manager = device_manager::Manager::new(&window);

    // Set up dear imgui
    let mut imgui = imgui::Context::create();
    let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    platform.attach_window(
        imgui.io_mut(),
        &window,
        imgui_winit_support::HiDpiMode::Default,
    );
    imgui.set_ini_filename(None);

    let font_size = (11.0 * hidpi_factor) as f32;
    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

    imgui.fonts().add_font(&[FontSource::DefaultFontData {
        config: Some(imgui::FontConfig {
            oversample_h: 1,
            pixel_snap_h: true,
            size_pixels: font_size,
            ..Default::default()
        }),
    }]);
    cpp_gui::ffi::init_imnodes();

    let mut renderer = Renderer::new(&mut imgui, &device_manager.device, &mut device_manager.queue, rendering::SWAPCHAIN_FORMAT);
    let mut last_frame = std::time::Instant::now();

    let mut last_cursor = None;

    let mut context = compute_chain::Context::default();
    //dbg!(&all_descriptors);
    let mut chain = match input_file {
        Some(filename) => {
            let mut json_contents = String::new();
            let mut file = std::fs::File::open(&filename).unwrap();
            file.read_to_string(&mut json_contents).unwrap();
            let json_scene: SceneDescriptor = serde_json::from_str(&json_contents).unwrap();
            context = json_scene.context;
            compute_chain::ComputeChain::create_from_descriptors(&device_manager.device, &json_scene.descriptors, &context).unwrap()
        }
        None => {
            print_usage(&program, opts);
            compute_chain::ComputeChain::new(&device_manager.device, &context)
        }
    };
    chain.run_chain(&device_manager.device, &device_manager.queue);

    //let dbg_buff = chain.chain.get("").expect("wrong block name for dbg printout").output_renderer.get_buffer();
    //let dbg_vect = copy_buffer_as_f32(out_buff, &device_manager.device);
    //println!("debugged buffer contains {:?}", dbg_vect);

    // let renderer = renderer::Renderer::new(&device_manager, out_buffer_slice);
    let mut scene_renderer = rendering::SurfaceRenderer::new(&device_manager);
    scene_renderer.update_renderables(&device_manager, &chain);

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
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit
                    },
                    WindowEvent::KeyboardInput { input, .. } => match input {
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        _ => {}
                    },
                    WindowEvent::Resized(physical_size) => {
                        device_manager.resize(*physical_size);
                    }
                    _ => {}
                }
        },
       Event::WindowEvent {
            event: WindowEvent::ScaleFactorChanged { scale_factor, .. },
            ..
        } => {
            hidpi_factor = scale_factor;
        },
        Event::RedrawRequested(_) => {
            // update variables and do the actual rendering
            // now, update the variables and run the chain again
            let time_var: &mut f32 = context.globals.get_mut(&"t".to_string()).unwrap();
            *time_var = elapsed_time.as_secs_f32();

            chain.update_globals(&device_manager.queue, &context);
            chain.run_chain(&device_manager.device, &device_manager.queue);
            let mut frame = device_manager.swap_chain.get_current_frame()
                .expect("could not get next frame");

            scene_renderer.render(&device_manager, &mut frame);

            // imgui stuff
            let delta_s = last_frame.elapsed();
            let now = std::time::Instant::now();
            imgui.io_mut().update_delta_time(now - last_frame);
            last_frame = now;

            //let frame = match device_manager.swap_chain.get_current_frame() {
            //    Ok(frame) => frame,
            //    Err(e) => {
            //        eprintln!("dropped frame: {:?}", e);
            //        return;
            //    }
            //};
            platform
                .prepare_frame(imgui.io_mut(), &window)
                .expect("Failed to prepare frame");
            let ui = imgui.frame();

            {
                cpp_gui::ffi::show_node_graph();
            }

            let mut encoder: wgpu::CommandEncoder =
                device_manager.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            if last_cursor != Some(ui.mouse_cursor()) {
                last_cursor = Some(ui.mouse_cursor());
                platform.prepare_render(&ui, &window);
            }

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.output.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            renderer
                .render(ui.render(), &device_manager.queue, &device_manager.device, &mut rpass)
                .expect("Rendering failed");

            drop(rpass);

            device_manager.queue.submit(Some(encoder.finish()));
        }
        Event::MainEventsCleared => {
            // RedrawRequested will only trigger once, unless we manually
            // request it.
            window.request_redraw();
        }
        Event::RedrawEventsCleared => {
        },
        _ => {}
        }
        platform.handle_event(imgui.io_mut(), &window, &event);
    });
}
