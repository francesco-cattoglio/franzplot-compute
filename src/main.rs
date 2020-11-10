use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
};
use imgui::*;
use imgui_wgpu::Renderer;
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
use camera::{ Camera, CameraController };
#[cfg(test)]
mod tests;

#[derive(Debug, Deserialize, Serialize)]
struct SceneDescriptor {
    global_vars: Vec<String>,
    descriptors: Vec<BlockDescriptor>,
}

use compute_block::*;
use getopts::Options;
use std::env;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

#[derive(Debug)]
pub enum CustomEvent {
    JsonScene(String),
    TestMessage(String),
    UpdateGlobals(Vec<(String, f32)>),
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

    wgpu_subscriber::initialize_default_subscriber(None);

    let event_loop = EventLoop::<CustomEvent>::with_user_event();
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

    let hidpi_factor = window.scale_factor();

    let mut device_manager = device_manager::Manager::new(&window);

    let mut camera = Camera::new(
        (-3.0, 0.0, 0.0).into(),
        0.0,
        0.0,
        device_manager.sc_desc.width as f32 / device_manager.sc_desc.height as f32,
        45.0,
        0.1,
        100.0,
    );
    let mut camera_controller = CameraController::new(4.0, 40.0);
    let mut last_mouse_pos = winit::dpi::PhysicalPosition::<f64>::new(0.0, 0.0);
    let mut mouse_pressed: bool = false;
    // Set up dear imgui
    let mut imgui = imgui::Context::create();
    let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    platform.attach_window(
        imgui.io_mut(),
        &window,
        imgui_winit_support::HiDpiMode::Default,
    );
    imgui.set_ini_filename(None);

    let font_size = (12.0 * hidpi_factor) as f32;
    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

    let glyph_range = FontGlyphRanges::from_slice(&[
        0x0020, 0x00FF, // Basic Latin + Latin Supplement
        0x2200, 0x22FF, // this range contains the miscellaneous symbols and arrows
        0x2600, 0x26FF, // miscelaneous symbols
        0]);
    imgui.fonts().add_font(&[FontSource::TtfData {
        data: include_bytes!("../resources/DejaVuSansCustom.ttf"),
        size_pixels: font_size,
        config: Some(imgui::FontConfig {
            oversample_h: 2,
            oversample_v: 2,
            pixel_snap_h: false,
            glyph_ranges: glyph_range,
            size_pixels: font_size,
            ..Default::default()
        }),
    }]);

    cpp_gui::ffi::init_imnodes();
    let mut gui_unique_ptr = cpp_gui::ffi::create_gui_instance(Box::new(event_loop.create_proxy()));

    let mut renderer = Renderer::new(&mut imgui, &device_manager.device, &device_manager.queue, rendering::SWAPCHAIN_FORMAT);
    let mut last_frame = std::time::Instant::now();

    let mut last_cursor = None;

    //dbg!(&all_descriptors);
    let mut chain = compute_chain::ComputeChain::new();
    let mut globals;
    if let Some(filename) = input_file {
        let mut json_contents = String::new();
        let mut file = std::fs::File::open(&filename).unwrap();
        file.read_to_string(&mut json_contents).unwrap();
        let json_scene: SceneDescriptor = serde_json::from_str(&json_contents).unwrap();
        globals = compute_chain::Globals::new(&device_manager.device, json_scene.global_vars);
                    let scene_result = chain.set_scene(&device_manager.device, &globals, json_scene.descriptors);
                    for (block_id, error) in scene_result.iter() {
                        let id = *block_id;
                        match error {
                            BlockCreationError::IncorrectAttributes(message) => {
                                println!("incorrect attributes error for {}: {}", id, &message);
                            },
                            BlockCreationError::InputNotBuilt(message) => {
                                println!("input not build warning for {}: {}", id, &message);
                            },
                            BlockCreationError::InputMissing(message) => {
                                println!("missing input error for {}: {}", id, &message);
                            },
                            BlockCreationError::InputInvalid(message) => {
                                println!("invalid input error for {}: {}", id, &message);
                            },
                            BlockCreationError::InternalError(message) => {
                                println!("internal error: {}", &message);
                                panic!();
                            },
                        }
                    }
    } else {
        globals = compute_chain::Globals::new(&device_manager.device, vec![]);
        print_usage(&program, opts);
    };
    chain.run_chain(&device_manager.device, &device_manager.queue, &globals);

    //let dbg_buff = chain.chain.get("").expect("wrong block name for dbg printout").output_renderer.get_buffer();
    //let dbg_vect = copy_buffer_as_f32(out_buff, &device_manager.device);
    //println!("debugged buffer contains {:?}", dbg_vect);

    // let renderer = renderer::Renderer::new(&device_manager, out_buffer_slice);
    let mut scene_renderer = rendering::Renderer::new(&device_manager);
    scene_renderer.update_renderables(&device_manager.device, &chain);

    let mut elapsed_time = std::time::Duration::from_secs(0);
    let mut old_instant = std::time::Instant::now();
    event_loop.run(move |event, _, control_flow| {
        let now = std::time::Instant::now();

        let frame_duration = now.duration_since(old_instant);
        if frame_duration.as_millis() > 0 {
            //println!("frame time: {} ms", frame_duration.as_millis());
            elapsed_time += frame_duration;
        }
        camera_controller.update_camera(&mut camera, frame_duration);
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
                        KeyboardInput {
                            virtual_keycode: Some(key),
                            state,
                            ..
                        } => camera_controller.process_keyboard(*key, *state),
                        _ => {}
                    },
                    WindowEvent::MouseInput {
                        button: winit::event::MouseButton::Left,
                        state,
                        ..
                    } => {
                        mouse_pressed = *state == ElementState::Pressed;
                    }
                    WindowEvent::CursorMoved {
                        position,
                        ..
                    } => {
                        let mouse_dx = position.x - last_mouse_pos.x;
                        let mouse_dy = position.y - last_mouse_pos.y;
                        last_mouse_pos = *position;
                        if mouse_pressed {
                            camera_controller.process_mouse(mouse_dx, mouse_dy);
                        }
                    }
                WindowEvent::Resized(physical_size) => {
                    device_manager.resize(*physical_size);
                }
                _ => {}
                }
        },
        Event::WindowEvent {
            event: WindowEvent::ScaleFactorChanged { .. },
            ..
        } => {
            // hidpi_factor = scale_factor;
        },
        Event::UserEvent(ref user_event) => {
            match user_event {
                CustomEvent::JsonScene(json_string) => {
                    let json_scene: SceneDescriptor = serde_jsonrc::from_str(&json_string).unwrap();
                    gui_unique_ptr.ClearAllMarks();
                    globals = compute_chain::Globals::new(&device_manager.device, json_scene.global_vars);
                    let scene_result = chain.set_scene(&device_manager.device, &globals, json_scene.descriptors);
                    scene_renderer.update_renderables(&device_manager.device, &chain);
                    for (block_id, error) in scene_result.iter() {
                        let id = *block_id;
                        match error {
                            BlockCreationError::IncorrectAttributes(message) => {
                                gui_unique_ptr.MarkError(id, message);
                                println!("incorrect attributes error for {}: {}", id, &message);
                            },
                            BlockCreationError::InputNotBuilt(message) => {
                                gui_unique_ptr.MarkWarning(id, message);
                                println!("input not build warning for {}: {}", id, &message);
                            },
                            BlockCreationError::InputMissing(message) => {
                                gui_unique_ptr.MarkError(id, message);
                                println!("missing input error for {}: {}", id, &message);
                            },
                            BlockCreationError::InputInvalid(message) => {
                                gui_unique_ptr.MarkError(id, message);
                                println!("invalid input error for {}: {}", id, &message);
                            },
                            BlockCreationError::InternalError(message) => {
                                println!("internal error: {}", &message);
                                panic!();
                            },
                        }
                    }
                }
                CustomEvent::TestMessage(string) => {
                    println!("the event loop received the following message: {}", string);
                }
                CustomEvent::UpdateGlobals(list) => {
                    globals.update(&device_manager.queue, list);
                }
            }
        },
        Event::RedrawRequested(_) => {
            // update variables and do the actual rendering
            // now, update the variables and run the chain again
            //let time_var: &mut f32 = context.globals.get_mut(&"t".to_string()).unwrap();
            //*time_var = elapsed_time.as_secs_f32();

            //// TODO: currently bugged due to "uneven" initialization of context
            //chain.update_globals(&device_manager.queue, &context);
            chain.run_chain(&device_manager.device, &device_manager.queue, &globals);
            let mut frame = device_manager.swap_chain.get_current_frame()
                .expect("could not get next frame");

            scene_renderer.render(&device_manager, &mut frame, &camera);

            // imgui stuff
            let _delta_s = last_frame.elapsed();
            let now = std::time::Instant::now();
            imgui.io_mut().update_delta_time(now - last_frame);
            last_frame = now;

            platform
                .prepare_frame(imgui.io_mut(), &window)
                .expect("Failed to prepare frame");
            let ui = imgui.frame();
            gui_unique_ptr.Render();

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
