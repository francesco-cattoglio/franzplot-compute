use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
};
use imgui::{FontSource, FontGlyphRanges};
use serde::{Deserialize, Serialize};

mod util;
mod camera;
mod texture;
mod rendering;
mod state;
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
    UpdateCamera(f32, f32),
    LockMouseCursor(u32, u32),
    UnlockMouseCursor,
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
    // TODO: camera controller movement currently depends on the frame dt. However,
    // rotation should NOT depend on it, since it depends on how many pixel I dragged
    // over the rendered scene, which kinda makes it already framerate-agnostic
    // OTOH, we might want to make it frame *dimension* agnostic!
    let mut camera_controller = CameraController::new(4.0, 0.1);
    // Set up dear imgui
    let mut imgui = imgui::Context::create();
    let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    // TODO: decide what to do about the hidpi. This requires a bit of investigation, especially
    // when we want to support both retina and small screen displays
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

    let mut renderer = imgui_wgpu::RendererConfig::new()
        .set_texture_format(rendering::SWAPCHAIN_FORMAT)
        .build(&mut imgui, &device_manager.device, &device_manager.queue);
    use wgpu::TextureUsage;
    let scene_texture = imgui_wgpu::TextureConfig::new(1280, 800)
        .set_label("scene texture config")
        .set_usage(TextureUsage::OUTPUT_ATTACHMENT | TextureUsage::SAMPLED | TextureUsage::COPY_DST)
        .build(&device_manager.device, &renderer);
    let scene_texture_id = renderer.textures.insert(scene_texture);
    gui_unique_ptr.UpdateSceneTexture(scene_texture_id.id());

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

    let mut scene_renderer = rendering::SceneRenderer::new(&device_manager);
    scene_renderer.update_renderables(&device_manager.device, &chain);

    // assemble the application state by moving all the variables inside it
    let mut app_state = state::State {
        camera,
        camera_controller,
        chain,
        globals,
        manager: device_manager,
        scene_renderer,
    };

    let mut frame_duration = std::time::Duration::from_secs(0);
    let mut old_instant = std::time::Instant::now();
    let mut frozen_mouse_position = winit::dpi::PhysicalPosition::new(0, 0);
    let mut freeze_mouse_position = false;
    event_loop.run(move |event, _, control_flow| {
        // BEWARE: keep in mind that if you go multi-window
        // you need to redo the whole handling of the events!
        // Please note: if you want to know in which order events
        // are dispatched to the handler, according to winit docs:
        // see https://docs.rs/winit/0.22.2/winit/event/index.html
        match event {
            // This event type is useful as a place to put code that should be done before you start processing events
            Event::NewEvents(_start_cause) => {
                let now = std::time::Instant::now();
                frame_duration = now.duration_since(old_instant);
                //println!("frame time: {} ms", frame_duration.as_millis());
                imgui.io_mut().update_delta_time(frame_duration); // this function only computes imgui internal time delta
                old_instant = now;
            }
            // Emitted when all of the event loop's input events have been processed and redraw processing is about to begin.
            Event::MainEventsCleared => {
                // update the chain
                app_state.chain.run_chain(&app_state.manager.device, &app_state.manager.queue, &app_state.globals);
                // prepare gui rendering
                app_state.camera_controller.update_camera(&mut app_state.camera, frame_duration);
                platform
                    .prepare_frame(imgui.io_mut(), &window)
                    .expect("Failed to prepare frame");
                window.request_redraw();
            }
            // Begin rendering. During each iteration of the event loop, Winit will aggregate duplicate redraw requests
            // into a single event, to help avoid duplicating rendering work.
            Event::RedrawRequested(_window_id) => {
                // redraw the scene to the texture
                if let Some(scene_texture) = renderer.textures.get(scene_texture_id) {
                    app_state.scene_renderer.render(&app_state.manager, scene_texture.view(), &app_state.camera);
                } else {
                    panic!("no texture for rendering!");
                }

                // get the framebuffer frame. We might need to re-create the swapchain if for some
                // reason our current one is outdated
                let maybe_frame = app_state.manager
                    .swap_chain
                    .get_current_frame();
                let frame = match maybe_frame {
                        Ok(swapchain_frame) => {
                            swapchain_frame
                        }
                        Err(wgpu::SwapChainError::Outdated) => {
                        // Recreate the swap chain to mitigate race condition on drawing surface resize.
                        // See https://github.com/parasyte/pixels/issues/121 and relevant fix:
                        // https://github.com/svenstaro/pixels/commit/b8b4fee8493a0d63d48f7dbc10032736022de677
                        app_state.manager.update_swapchain(&window);
                        app_state.manager
                            .swap_chain
                            .get_current_frame()
                            .unwrap()
                        }
                        Err(wgpu::SwapChainError::OutOfMemory) => {
                            panic!("Out Of Memory error in frame rendering");
                        }
                        Err(wgpu::SwapChainError::Timeout) => {
                            panic!("Timeout error in frame rendering");
                        }
                        Err(wgpu::SwapChainError::Lost) => {
                            panic!("Frame Lost error in frame rendering");
                        }
                };

                // use the acquired frame for a new rendering pass, which will clear the screen and
                // render imgui
                let mut encoder: wgpu::CommandEncoder =
                    app_state.manager.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &frame.output.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                });

                // actual imgui rendering
                let ui = imgui.frame();
                let size = window.inner_size().to_logical(hidpi_factor);
                gui_unique_ptr.Render(size.width, size.height);
                platform.prepare_render(&ui, &window);
                renderer
                    .render(ui.render(), &app_state.manager.queue, &app_state.manager.device, &mut rpass)
                    .expect("Imgui rendering failed");

                drop(rpass);

                // submit the framebuffer rendering pass
                app_state.manager.queue.submit(Some(encoder.finish()));
            }
            // Emitted after all RedrawRequested events have been processed and control flow is about to be taken away from the program.
            // If there are no RedrawRequested events, it is emitted immediately after MainEventsCleared.
            Event::RedrawEventsCleared => {
                // If we are dragging onto something that requires the mouse pointer to stay fixed,
                // this is the moment in which we move it back to its old position.
                if freeze_mouse_position {
                    window.set_cursor_position(frozen_mouse_position);
                }
            }
            // Emitted when an event is sent from EventLoopProxy::send_event
            // This is where we handle all the events generated in our cpp gui
            // TODO: maybe move this somewhere else, it does look already massive and will grow even more
            // over time
            Event::UserEvent(user_event) => {
                match user_event {
                    CustomEvent::JsonScene(json_string) => {
                        let json_scene: SceneDescriptor = serde_jsonrc::from_str(&json_string).unwrap();
                        gui_unique_ptr.ClearAllMarks();
                        app_state.globals = compute_chain::Globals::new(&app_state.manager.device, json_scene.global_vars);
                        let scene_result = app_state.chain.set_scene(&app_state.manager.device, &app_state.globals, json_scene.descriptors);
                        app_state.scene_renderer.update_renderables(&app_state.manager.device, &app_state.chain);
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
                        app_state.globals.update(&app_state.manager.queue, &list);
                    }
                    CustomEvent::UpdateCamera(dx, dy) => {
                        app_state.camera_controller.process_mouse(dx, dy);
                    }
                    CustomEvent::LockMouseCursor(x, y) => {
                        frozen_mouse_position = winit::dpi::LogicalPosition::new(x, y).to_physical(hidpi_factor);
                        window.set_cursor_visible(false);
                        freeze_mouse_position = true;
                    }
                    CustomEvent::UnlockMouseCursor => {
                        freeze_mouse_position = false;
                        window.set_cursor_visible(true);
                    }
                }
            }
            // match a very specific WindowEvent: user-requested closing of the application
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            // catch-all for remaining events (WindowEvent and DeviceEvent). We do this because
            // we want imgui to handle it first, and then do any kind of "post-processing"
            // that we might be thinking of.
            other_event => {
                // in here, imgui will process keyboard and mouse status!
                platform.handle_event(imgui.io_mut(), &window, &other_event);

                // "post-processing" of input
                match other_event {
                    // if the window was resized, we need to resize the swapchain as well!
                    Event::WindowEvent{ event: WindowEvent::Resized(physical_size), .. } => {
                        app_state.manager.resize(physical_size);
                    }
                    Event::WindowEvent{ event: WindowEvent::MouseInput { button, state, .. }, ..} => {
                        // safety un-freeze feature, in case the cpp gui messes something up really bad
                        if button == MouseButton::Left && state == ElementState::Released {
                            freeze_mouse_position = false;
                            window.set_cursor_visible(true);
                        }
                    }
                    Event::WindowEvent{ event: WindowEvent::KeyboardInput { input, .. }, .. } => {
                        if input.state == ElementState::Pressed && input.virtual_keycode == Some(VirtualKeyCode::Escape) {
                            *control_flow = ControlFlow::Exit;
                        }
                    }

                    _ => {}
                }
            }
        }
    });
}
