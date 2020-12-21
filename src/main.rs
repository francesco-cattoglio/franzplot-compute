use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
};

use imgui::{FontSource, FontGlyphRanges};

mod util;
mod rendering;
mod state;
mod computable_scene;
mod device_manager;
mod shader_processing;
mod node_graph;
mod rust_gui;
mod cpp_gui;
mod file_io;
#[cfg(test)]
mod tests;

use getopts::Options;
use std::env;


fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

#[allow(unused)]
#[derive(Debug)]
pub enum CustomEvent {
    OpenFile(std::path::PathBuf),
    SaveFile(std::path::PathBuf),
    MouseFreeze,
    MouseThaw,
    CurrentlyUnused,
}

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

    let _input_file = matches.opt_str("i");

    // wgpu_subscriber::initialize_default_subscriber(None);

    let event_loop = EventLoop::<CustomEvent>::with_user_event();
    let mut builder = winit::window::WindowBuilder::new();
    // TODO: if you try using fixed dimensions that are too big for the screen to fit
    // (eg: a 768p monitor) the returned window will have different size and
    // the program crashes because the scene texture size will not match
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

    let device_manager = device_manager::Manager::new(&window);

    // Set up dear imgui
    let mut imgui = imgui::Context::create();
    imgui.style_mut().window_rounding = 0.0;
    let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    // TODO: decide what to do about the hidpi. This requires a bit of investigation, especially
    // when we want to support both retina and small screen displays
    platform.attach_window(
        imgui.io_mut(),
        &window,
        imgui_winit_support::HiDpiMode::Default,
    );
    imgui.set_ini_filename(None);

    let mut camera_inputs = rendering::camera::InputState::default();
    let mut cursor_position = winit::dpi::PhysicalPosition::<f64>::new(0.0, 0.0);
    let mut mouse_frozen = false;

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

    cpp_gui::imnodes::Initialize();

    let renderer_config = imgui_wgpu::RendererConfig {
        texture_format: rendering::SWAPCHAIN_FORMAT,
        .. Default::default()
    };
    let mut renderer = imgui_wgpu::Renderer::new(&mut imgui, &device_manager.device, &device_manager.queue, renderer_config);

    // first, create a texture that will be used to render the scene and display it inside of imgui
    let scene_texture = rendering::texture::Texture::create_output_texture(&device_manager.device, wgpu::Extent3d::default(), 1);
    let scene_texture_id = renderer.textures.insert(scene_texture.into());

    // then, load all masks that will be available in the rendering node and push them to imgui
    // BEWARE: if you change the number of masks, you also need to modify the MaskIds in
    // rust_gui.rs and the Masks in texture.rs!
    let mask_paths: [&str; 4] = [
        "./resources/masks/checker_8.png",
        "./resources/masks/h_stripes_16.png",
        "./resources/masks/v_stripes_16.png",
        "./resources/masks/blank.png"
    ];
    let dir_files = std::fs::read_dir("./resources/materials/")
        .unwrap(); // unwraps the dir reading, giving an iterator over its files

    // process the dir files, returning only valid filenames that end in "png"
    let mut material_files: Vec<std::path::PathBuf> = dir_files
        .filter_map(|dir_result| {
            let dir_entry = dir_result.ok()?;
            let path = dir_entry.path();
            if !path.is_file() {
                return None;
            }
            let extension = path.extension()?.to_str()?.to_lowercase();
            if extension == *"png" {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    material_files.sort();
    dbg!(&material_files);

    let imgui_masks = util::load_imgui_masks(&device_manager, &mut renderer, &mask_paths);
    let imgui_materials = util::load_imgui_materials(&device_manager, &mut renderer, &material_files);

    let masks = util::load_masks(&device_manager, &mask_paths);
    let materials = util::load_materials(&device_manager, &material_files);

    // last, initialize the rust_gui and the state with the list of available masks and materials.
    let mut rust_gui = rust_gui::Gui::new(event_loop.create_proxy(), scene_texture_id, imgui_masks, imgui_materials);
    let mut state = state::State::new(device_manager, masks, materials);

    let mut old_instant = std::time::Instant::now();
    let mut modifiers_state = winit::event::ModifiersState::default();


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
                let frame_duration = now.duration_since(old_instant);
                //println!("frame time: {} ms", frame_duration.as_millis());
                imgui.io_mut().update_delta_time(frame_duration); // this function only computes imgui internal time delta
                old_instant = now;
            }
            // Emitted when all of the event loop's input events have been processed and redraw processing is about to begin.
            Event::MainEventsCleared => {
                // prepare gui rendering
                platform
                    .prepare_frame(imgui.io_mut(), &window)
                    .expect("Failed to prepare frame");
                window.request_redraw();
            }
            // Begin rendering. During each iteration of the event loop, Winit will aggregate duplicate redraw requests
            // into a single event, to help avoid duplicating rendering work.
            Event::RedrawRequested(_window_id) => {
                // acquire next frame, or update the swapchain if a resize occurred
                let frame = state.app.manager.get_frame_or_update(&window);

                // use the acquired frame for a rendering pass, which will clear the screen and render the gui
                let mut encoder: wgpu::CommandEncoder =
                    state.app.manager.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

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
                let requested_logical_size = rust_gui.render(&ui, [size.width, size.height], &mut state);
                let requested_physical_size : Option<winit::dpi::PhysicalSize<u32>> = requested_logical_size
                    .map(|imgui_size| {
                        let logical_size = winit::dpi::LogicalSize::new(imgui_size[0], imgui_size[1]);
                        logical_size.to_physical(hidpi_factor)
                    });
                // after the GUI render command, we know if we need to render the scene or not
                if let Some(physical_size) = requested_physical_size {
                    let scene_texture = renderer.textures.get(scene_texture_id).unwrap();
                    // first, check if the scene size has changed. If so, re-create the scene
                    // texture and depth buffer
                    if physical_size.width != scene_texture.width() || physical_size.height != scene_texture.height() {
                        let texture_size = wgpu::Extent3d {
                            height: physical_size.height,
                            width: physical_size.width,
                            depth: 1,
                        };
                        dbg!(texture_size);
                        dbg!(requested_logical_size);
                        state.app.update_depth_buffer(texture_size);
                        state.app.update_projection_matrix(texture_size);
                        let new_scene_texture = rendering::texture::Texture::create_output_texture(&state.app.manager.device, texture_size, 1);
                        renderer.textures.replace(scene_texture_id, new_scene_texture.into()).unwrap();
                    }
                    // update the scene
                    let scene_texture_view = renderer.textures.get(scene_texture_id).unwrap().view();
                    state.app.update_scene(scene_texture_view, &camera_inputs);
                }

                platform.prepare_render(&ui, &window);
                renderer
                    .render(ui.render(), &state.app.manager.queue, &state.app.manager.device, &mut rpass)
                    .expect("Imgui rendering failed");

                drop(rpass); // dropping the render pass is required for the encoder.finish() command

                // submit the framebuffer rendering pass
                state.app.manager.queue.submit(Some(encoder.finish()));
            }
            // Emitted after all RedrawRequested events have been processed and control flow is about to be taken away from the program.
            // If there are no RedrawRequested events, it is emitted immediately after MainEventsCleared.
            Event::RedrawEventsCleared => {
                // If we are dragging onto something that requires the mouse pointer to stay fixed,
                // this is the moment in which we move it back to its old position.
                if mouse_frozen {
                    window.set_cursor_position(cursor_position).unwrap();
                }
                camera_inputs.reset_deltas();
            }
            // Emitted when an event is sent from EventLoopProxy::send_event
            // We are not currently using it, but this might become useful for issuing commands
            // to winit that have to be executed during the next frame.
            Event::UserEvent(user_event) => {
                match user_event {
                    CustomEvent::SaveFile(path_buf) => {
                        state.user.write_to_file(&path_buf);
                    },
                    CustomEvent::OpenFile(path_buf) => {
                        state.user.read_from_file(&path_buf);
                        rust_gui.issue_savestate(&mut state, imgui.time());
                    },
                    CustomEvent::MouseFreeze => {
                        mouse_frozen = true;
                    },
                    CustomEvent::MouseThaw => {
                        mouse_frozen = false;
                    },
                    CustomEvent::CurrentlyUnused => println!("received a custom user event")
                }
            }
            // Emitted when the event loop is being shut down.
            Event::LoopDestroyed => {
                cpp_gui::imnodes::Shutdown();
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

                // additional processing of input
                match other_event {
                    // if the window was resized, we need to resize the swapchain as well!
                    Event::WindowEvent{ event: WindowEvent::Resized(physical_size), .. } => {
                        state.app.manager.resize(physical_size);
                    }
                    Event::WindowEvent{ event: WindowEvent::CursorMoved { position, .. }, ..} => {
                        // put a safety un-freeze feature, in case we mess something up wrt releasing the mouse
                        if !mouse_frozen {
                            cursor_position = position;
                        }
                    }
                    Event::WindowEvent{ event: WindowEvent::MouseInput { .. }, ..} => {
                        // put a safety un-freeze feature, in case we mess something up wrt releasing the mouse
                    }
                    Event::WindowEvent{ event: WindowEvent::ModifiersChanged(modifiers), ..} => {
                        modifiers_state = modifiers;
                    }
                    // shortcuts processing goes here
                    Event::WindowEvent{ event: WindowEvent::KeyboardInput { input, .. }, .. } => {
                        if input.state == ElementState::Pressed && input.virtual_keycode == Some(VirtualKeyCode::Escape) {
                            *control_flow = ControlFlow::Exit;
                        } else if input.state == ElementState::Pressed && input.virtual_keycode == Some(VirtualKeyCode::Z) {
                            if modifiers_state.ctrl() && modifiers_state.shift() {
                                rust_gui.issue_redo(&mut state);
                            } else if modifiers_state.ctrl() {
                                rust_gui.issue_undo(&mut state, imgui.time());
                            }
                        }
                    }
                    Event::DeviceEvent{ event: DeviceEvent::MouseMotion { delta }, ..} => {
                        camera_inputs.mouse_motion = delta;
                    }
                    Event::DeviceEvent{ event: DeviceEvent::MouseWheel { delta }, ..} => {
                        camera_inputs.mouse_wheel = delta;
                    }
                    Event::DeviceEvent{ event: DeviceEvent::Button {button, state }, .. } => {
                        let pressed = state == ElementState::Pressed;
                        match button {
                            1 => camera_inputs.mouse_left_click = pressed,
                            3 => camera_inputs.mouse_right_click = pressed,
                            _ => {},
                        }
                    }

                    _ => {}
                }
            }
        }
    });
}
