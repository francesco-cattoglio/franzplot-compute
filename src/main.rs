extern crate pest;
#[macro_use]
extern crate pest_derive;

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
mod parser;
#[cfg(test)]
mod tests;

use std::env;

#[allow(unused)]
#[derive(Debug)]
pub enum CustomEvent {
    NewFile,
    ShowOpenDialog,
    OpenFile(std::path::PathBuf),
    SaveFile(std::path::PathBuf),
    ExportGraphPng(std::path::PathBuf),
    ExportScenePng(std::path::PathBuf),
    RequestExit,
    MouseFreeze,
    MouseThaw,
}

pub struct PhysicalRectangle {
    position: winit::dpi::PhysicalPosition::<i32>,
    size: winit::dpi::PhysicalSize::<u32>,
}

impl PhysicalRectangle {
    fn from_imgui_rectangle(rectangle: &rust_gui::SceneRectangle, hidpi_factor: f64) -> Self {
        let logical_pos = winit::dpi::LogicalPosition::new(rectangle.position[0], rectangle.position[1]);
        let logical_size = winit::dpi::LogicalSize::new(rectangle.size[0], rectangle.size[1]);
        PhysicalRectangle {
            position: logical_pos.to_physical(hidpi_factor),
            size: logical_size.to_physical(hidpi_factor),
        }
    }
}

fn add_custom_font(imgui_context: &mut imgui::Context, font_size: f32) -> imgui::FontId {
    let glyph_range = FontGlyphRanges::from_slice(&[
        0x0020, 0x00FF, // Basic Latin + Latin Supplement
        0x2200, 0x22FF, // this range contains the miscellaneous symbols and arrows
        0x2600, 0x26FF, // miscelaneous symbols
        0]);
    imgui_context.fonts().add_font(&[FontSource::TtfData {
        data: include_bytes!("../compile_resources/DejaVuSansCustom.ttf"),
        size_pixels: font_size,
        config: Some(imgui::FontConfig {
            oversample_h: 2,
            oversample_v: 2,
            pixel_snap_h: false,
            glyph_ranges: glyph_range,
            size_pixels: font_size,
            ..Default::default()
        }),
    }])
}

fn main() {
    let matches = clap::App::new("Franzplot")
        .version(clap::crate_version!())
        .author("Francesco Cattoglio")
        .about("a tool to create and visualize parametric curves and surfaces")
        .arg(clap::Arg::with_name("INPUT")
            .help("Open an existing file instead of starting from a new one")
            .required(false)
            .index(1))
        .arg(clap::Arg::with_name("export")
             .help("Instead of running the program normally, FranzPlot will only export pictures of the graph and the scene.
                   The name of the output files will be <export_value>.graph.png and <export_value.scene.png>")
             .short("e")
             .long("export")
             .value_name("EXPORT")
             .takes_value(true))
        .arg(clap::Arg::with_name("tracing")
            .short("t")
            .long("tracing")
            .value_name("TRACE_PATH")
            .help("Sets a path for tracing output")
            .takes_value(true))
        .arg(clap::Arg::with_name("backend")
            .short("b")
            .long("backend")
            .value_name("WGPU_BACKEND")
            .help("Chose a different backend from the standard one")
            .takes_value(true))
        .arg(clap::Arg::with_name("p")
            .short("p")
            .multiple(true)
            .help("Pause the execution before creating a device. Useful for attaching a debugger to a process. Each p is 5 secs"))
        .get_matches();

    //wgpu_subscriber::initialize_default_subscriber(None);

    use std::{thread, time};
    let seconds_to_wait = 5 * matches.occurrences_of("p");
    thread::sleep(time::Duration::from_secs(seconds_to_wait));

    let maybe_tracing_path = matches.value_of("tracing");
    let tracing_path_option = maybe_tracing_path.map(|x| std::path::Path::new(x));

    let maybe_input_file = matches.value_of("INPUT");

    let maybe_export_path = matches.value_of("export");
    let exporting_hang_workaround = maybe_export_path.is_some();

    let maybe_backend = matches.value_of("backend").map(|name| {
        match name.to_lowercase().as_str()  {
            "vulkan" => wgpu::Backend::Vulkan,
            "metal" => wgpu::Backend::Metal,
            "dx12" => wgpu::Backend::Dx12,
            "dx11" => wgpu::Backend::Dx11,
            "gl" => wgpu::Backend::Gl,
            "webgpu" => wgpu::Backend::BrowserWebGpu,
            other => panic!("Unknown backend: {}", other),
        }
    });

    let event_loop = EventLoop::<CustomEvent>::with_user_event();

    let window_size = if let Some(monitor) = event_loop.primary_monitor() {
        // web winit always reports a size of zero
        #[cfg(not(target_arch = "wasm32"))]
        let screen_size = monitor.size();
        if maybe_export_path.is_some() {
            winit::dpi::PhysicalSize::new(screen_size.width, screen_size.height)
        } else {
            winit::dpi::PhysicalSize::new(screen_size.width * 3 / 4, screen_size.height * 3 / 4)
        }
    } else {
        winit::dpi::PhysicalSize::new(1280, 800)
    };

    let icon_png = include_bytes!("../compile_resources/icon_128.png");
    let icon_image = image::load_from_memory(icon_png).expect("Bad icon png file");
    let (icon_rgba, icon_width, icon_height) = {
        let image = icon_image.into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    let icon = winit::window::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Bad icon format");

    let mut builder = winit::window::WindowBuilder::new();
    builder = builder
        .with_title("test")
        .with_window_icon(Some(icon))
        .with_inner_size(window_size);
    if maybe_export_path.is_some() {
        builder = builder.with_visible(false);
    }
    #[cfg(windows_OFF)] // TODO check for news regarding this
    {
        use winit::platform::windows::WindowBuilderExtWindows;
        builder = builder.with_no_redirection_bitmap(true);
    }
    let window = builder.build(&event_loop).unwrap();

    let hidpi_factor = window.scale_factor();
    window.set_min_inner_size(Some(winit::dpi::LogicalSize::new(200.0, 100.0)));
    let device_manager = device_manager::Manager::new(&window, tracing_path_option, maybe_backend);

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
    let mut cursor_position = winit::dpi::PhysicalPosition::<i32>::new(0, 0);
    let mut mouse_frozen = false;

    let font_size = (12.0 * hidpi_factor) as f32;
    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;
    add_custom_font(&mut imgui, font_size);

    let graph_fonts: Vec<imgui::FontId> = node_graph::ZOOM_LEVELS.iter()
        .map(|scale| add_custom_font(&mut imgui, 12.0 * hidpi_factor as f32 * scale))
        .collect();
    cpp_gui::imnodes::Initialize();
    cpp_gui::imnodes::EnableCtrlScroll(true, &imgui.io().key_ctrl);

    let renderer_config = imgui_wgpu::RendererConfig {
        texture_format: rendering::SWAPCHAIN_FORMAT,
        .. Default::default()
    };
    let mut renderer = imgui_wgpu::Renderer::new(&mut imgui, &device_manager.device, &device_manager.queue, renderer_config);

    // first, create a texture that will be used to render the scene and display it inside of imgui
    let scene_texture = rendering::texture::Texture::create_output_texture(&device_manager.device, wgpu::Extent3d::default(), 1);
    let scene_texture_id = renderer.textures.insert(scene_texture.into());

    let resources_path = {
        // Check if the resources folder can be find as a subfolder of current work directory
        let mut dir_path = env::current_dir().unwrap();
        dir_path.push("resources");
        let mut exe_path = env::current_exe().unwrap();
        exe_path.pop();
        exe_path.push("resources");
        if dir_path.is_dir() {
            dir_path
        } else if exe_path.is_dir() {
            exe_path
        } else {
            panic!("Could not find the 'resources' folder");
        }
    };
    let mut masks_dir = resources_path.clone();
    masks_dir.push("masks");
    let mut materials_dir = resources_path.clone();
    materials_dir.push("materials");

    // then, load all masks that will be available in the rendering node and push them to imgui
    // BEWARE: if you change the number of masks, you also need to modify the MaskIds in
    // rust_gui.rs and the Masks in texture.rs!
    let mask_names: [&str; 5] = [
        "checker_8.png",
        "h_stripes_16.png",
        "v_stripes_16.png",
        "blank.png",
        "alpha_grid.png",
    ];
    use std::convert::TryInto;
    let mask_files: [std::path::PathBuf; 5] = mask_names
        .iter()
        .map(|name| {
            let mut mask_path = masks_dir.clone();
            mask_path.push(name);
            mask_path
        })
        .collect::<Vec<_>>() // make it into a vector
        .try_into() // and then turn it into an array
        .unwrap(); // panic if dimensions don't match

    // process the materials_dir files, returning only valid filenames that end in "png"
    let materials_dir_files = std::fs::read_dir(materials_dir)
        .unwrap(); // unwraps the dir reading, giving an iterator over its files
    let mut material_files: Vec<std::path::PathBuf> = materials_dir_files
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

    let imgui_masks = util::load_imgui_masks(&device_manager, &mut renderer, &mask_files);
    let imgui_materials = util::load_imgui_materials(&device_manager, &mut renderer, &material_files);

    let masks = util::load_masks(&device_manager, &mask_files);
    let materials = util::load_materials(&device_manager, &material_files);
    assert!(materials.len() > 0, "Error while loading resources: could not load any material.");

    // do the same for models
    let mut models_dir = resources_path.clone();
    models_dir.push("models");
    let models_dir_files = std::fs::read_dir(models_dir)
        .unwrap(); // unwraps the dir reading, giving an iterator over its files

    // process the dir files, returning only valid filenames that end in "obj"
    let mut model_files: Vec<std::path::PathBuf> = models_dir_files
        .filter_map(|dir_result| {
            let dir_entry = dir_result.ok()?;
            let path = dir_entry.path();
            if !path.is_file() {
                return None;
            }
            let extension = path.extension()?.to_str()?.to_lowercase();
            if extension == *"obj" {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    model_files.sort();
    let models = util::load_models(&device_manager.device, &model_files);
    let model_names = util::imgui_model_names(&model_files);
    assert!(models.len() > 0, "Error while loading resources: could not load any model.");

    let assets = state::Assets {
        materials,
        models,
        masks,
    };

    // last, initialize the rust_gui and the state with the available assets.
    let availables = rust_gui::Availables {
        mask_ids: imgui_masks,
        material_ids: imgui_materials,
        model_names,
    };
    let executor = util::Executor::new();
    let event_loop_proxy = event_loop.create_proxy();
    let mut rust_gui = rust_gui::Gui::new(event_loop.create_proxy(), scene_texture_id, availables, graph_fonts);
    let mut state = state::State::new(device_manager, assets);

    if let Some(file) = maybe_input_file {
        let file_path = std::path::PathBuf::from(file);
        println!("FranzPlot starting. Opening file: {}", file);
        event_loop_proxy.send_event(CustomEvent::OpenFile(file_path)).unwrap();
    } else {
        println!("FranzPlot starting, no file selected.");
    }

    if let Some(path) = maybe_export_path {
        let scene_file_name = String::new() + path + ".scene.png";
        let scene_file_path = std::path::PathBuf::from(&scene_file_name);
        let graph_file_name = String::new() + path + ".graph.png";
        let graph_file_path = std::path::PathBuf::from(&graph_file_name);
        state.app.camera.set_x1_y1_z1_wide();
        event_loop_proxy.send_event(CustomEvent::ExportGraphPng(graph_file_path)).unwrap();
        event_loop_proxy.send_event(CustomEvent::ExportScenePng(scene_file_path)).unwrap();
        event_loop_proxy.send_event(CustomEvent::RequestExit).unwrap();
    }

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
                let frame = if let Some(frame) = state.app.manager.get_frame() {
                    frame
                } else {
                    // if we are unable to get a frame, skip rendering altogether
                    return;
                };

                // use the acquired frame for a rendering pass, which will clear the screen and render the gui
                let mut encoder: wgpu::CommandEncoder =
                    state.app.manager.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[wgpu::RenderPassColorAttachment {
                        view: &frame.output.view,
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
                let requested_logical_rectangle = rust_gui.render(&ui, [size.width, size.height], &mut state, &executor);
                // after calling the gui render function we know if we need to render the scene or not
                if let Some(logical_rectangle) = requested_logical_rectangle {
                    let physical_rectangle = PhysicalRectangle::from_imgui_rectangle(&logical_rectangle, hidpi_factor);
                    let scene_texture = renderer.textures.get(scene_texture_id).unwrap();
                    // first, check if the scene size has changed. If so, re-create the scene
                    // texture and depth buffer
                    if (physical_rectangle.size.width != scene_texture.width() || physical_rectangle.size.height != scene_texture.height())
                            && (physical_rectangle.size.width > 8 && physical_rectangle.size.height > 8) {
                        let texture_size = wgpu::Extent3d {
                            height: physical_rectangle.size.height,
                            width: physical_rectangle.size.width,
                            depth_or_array_layers: 1,
                        };
                        state.app.update_depth_buffer(texture_size);
                        state.app.update_projection_matrix(texture_size);
                        let new_scene_texture = rendering::texture::Texture::create_output_texture(&state.app.manager.device, texture_size, 1);
                        renderer.textures.replace(scene_texture_id, new_scene_texture.into()).unwrap();
                    }
                    // update the scene
                    let scene_texture_view = renderer.textures.get(scene_texture_id).unwrap().view();

                    let relative_pos = [
                        cursor_position.x - physical_rectangle.position.x,
                        cursor_position.y - physical_rectangle.position.y,
                    ];
                    state.app.computable_scene.renderer.update_mouse_pos(&relative_pos);
                    state.app.update_camera(&camera_inputs);
                    state.app.update_scene(scene_texture_view);
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
                    #[cfg(target_os = "windows")]
                    {
                        window.set_cursor_position(cursor_position).unwrap();
                    }

                    #[cfg(target_os = "linux")]
                    {
                        window.set_cursor_position(cursor_position).unwrap();
                    }

                    #[cfg(target_os = "macos")]
                    {}
                }
                camera_inputs.reset_deltas();
            }
            // Emitted when an event is sent from EventLoopProxy::send_event
            Event::UserEvent(user_event) => {
                match user_event {
                    CustomEvent::ShowOpenDialog => {
                        file_io::async_pick_open(event_loop_proxy.clone(), &executor);
                    },
                    CustomEvent::NewFile => {
                        state.new_file();
                        rust_gui.reset_undo_history(&mut state);
                        rust_gui.reset_nongraph_data();
                    },
                    CustomEvent::RequestExit => {
                        *control_flow = ControlFlow::Exit;
                        if exporting_hang_workaround {
                            std::process::exit(0);
                        }
                    },
                    CustomEvent::SaveFile(path_buf) => {
                        state.write_to_frzp(&path_buf);
                        rust_gui.graph_edited = false;
                    },
                    CustomEvent::OpenFile(path_buf) => {
                        match state.read_from_frzp(&path_buf) {
                            Ok(()) => {
                                rust_gui.reset_undo_history(&mut state);
                                rust_gui.reset_nongraph_data();
                                rust_gui.opened_tab[0] = true;
                            },
                            Err(error) => {
                                file_io::async_dialog_failure(&executor, error);
                            }
                        }
                    },
                    CustomEvent::ExportGraphPng(path_buf) => {
                        println!("Exporting graph: {:?}", &path_buf);
                        // zoom out once or twice
                        state.user.graph.zoom_down_graph([0.0, 0.0]);
                        //state.user.graph.zoom_down_graph([0.0, 0.0]);
                        state.user.graph.push_all_to_corner();
                        state.user.graph.push_positions_to_imnodes();
                        util::create_graph_png(&mut state, &path_buf,&window,&mut platform,&mut renderer,&mut rust_gui,&mut imgui, window_size.to_logical(hidpi_factor));
                    },
                    CustomEvent::ExportScenePng(path_buf) => {
                        println!("Exporting scene: {:?}", &path_buf);
                        util::create_scene_png(&mut state, &path_buf);
                        // TODO: this is hacked in and needs some refactoring:
                        // util::create_png modifies the state changing the depth buffer and the
                        // projection because we run the rendering on the output png size.
                        // before we continue with our normal execution we need to resize
                        // everything back
                        let scene_texture = renderer.textures.get(scene_texture_id).unwrap();
                        let texture_size = wgpu::Extent3d {
                            height: scene_texture.height(),
                            width: scene_texture.width(),
                            depth_or_array_layers: 1,
                        };
                        state.app.update_depth_buffer(texture_size);
                        state.app.update_projection_matrix(texture_size);
                    },
                    CustomEvent::MouseFreeze => {
                        // set mouse as frozen
                        mouse_frozen = true;
                        window.set_cursor_visible(false);
                        #[cfg(target_os = "windows")]
                        {}

                        #[cfg(target_os = "linux")]
                        {
                        }

                        #[cfg(target_os = "macos")]
                        {
                            window.set_cursor_grab(true).unwrap();
                        }
                    },
                    CustomEvent::MouseThaw => {
                        mouse_frozen = false;
                        window.set_cursor_visible(true);
                        #[cfg(target_os = "windows")]
                        {}

                        #[cfg(target_os = "linux")]
                        {}

                        #[cfg(target_os = "macos")]
                        {
                            window.set_cursor_grab(false).unwrap();
                        }
                    },
                }
            }
            // Emitted when the event loop is being shut down.
            Event::LoopDestroyed => {
                cpp_gui::imnodes::Shutdown();
            }
            // match a very specific WindowEvent: user-requested closing of the application
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                if rust_gui.graph_edited {
                    file_io::async_confirm_exit(event_loop_proxy.clone(), &executor)
                } else {
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::WindowEvent { event: WindowEvent::DroppedFile(file_path), .. } => {
                if rust_gui.graph_edited {
                    file_io::async_confirm_load(event_loop_proxy.clone(), &executor, file_path);
                } else {
                    event_loop_proxy.send_event(CustomEvent::OpenFile(file_path)).unwrap();
                }
            },
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
                        if !mouse_frozen {
                            cursor_position = position.cast();
                        }
                    }
                    Event::WindowEvent{ event: WindowEvent::ModifiersChanged(modifiers), ..} => {
                        modifiers_state = modifiers;
                    }
                    // shortcuts and other keyboard processing goes here
                    Event::WindowEvent{ event: WindowEvent::KeyboardInput { input, .. }, .. } => {
                        if input.state == ElementState::Pressed && input.virtual_keycode == Some(VirtualKeyCode::Z) {
                            if modifiers_state.ctrl() && modifiers_state.shift() {
                                rust_gui.issue_redo(&mut state);
                            } else if modifiers_state.ctrl() {
                                rust_gui.issue_undo(&mut state, imgui.time());
                            }
                        }
                        if input.state == ElementState::Pressed && input.virtual_keycode == Some(VirtualKeyCode::Key1) {
                            camera_inputs.reset_to_xz = true;
                        }
                        if input.state == ElementState::Pressed && input.virtual_keycode == Some(VirtualKeyCode::Key2) {
                            camera_inputs.reset_to_yz = true;
                        }
                        if input.state == ElementState::Pressed && input.virtual_keycode == Some(VirtualKeyCode::Key3) {
                            camera_inputs.reset_to_xy = true;
                        }
                        if input.state == ElementState::Pressed && input.virtual_keycode == Some(VirtualKeyCode::Key4) {
                            camera_inputs.reset_to_xyz = true;
                        }
                        if input.state == ElementState::Pressed && input.virtual_keycode == Some(VirtualKeyCode::Key5) {
                            camera_inputs.reset_to_minus_xz = true;
                        }
                        if input.state == ElementState::Pressed && input.virtual_keycode == Some(VirtualKeyCode::Key6) {
                            camera_inputs.reset_to_minus_yz = true;
                        }
                        if input.state == ElementState::Pressed && input.virtual_keycode == Some(VirtualKeyCode::Key7) {
                            camera_inputs.reset_to_minus_xy = true;
                        }
                    }
                    Event::DeviceEvent{ event: DeviceEvent::MouseMotion { delta }, ..} => {
                        // Since we might receive many different mouse motion events in
                        // the same frame, the correct thing to do is to accumulate them
                        camera_inputs.mouse_motion.0 += delta.0;
                        camera_inputs.mouse_motion.1 += delta.1;
                    }
                    Event::WindowEvent{ event: WindowEvent::MouseWheel { delta, .. }, ..} => {
                        let sensitivity = &state.app.sensitivity;
                        camera_inputs.mouse_wheel = util::compute_scene_zoom(delta, sensitivity.scene_zoom);
                        rust_gui.added_zoom = util::compute_graph_zoom(delta, sensitivity.graph_zoom);
                    }
                    Event::WindowEvent{ event: WindowEvent::MouseInput { state, button, .. }, ..} => {
                        // BEWARE: the `state` variable in this scope shadows the "application state" variable
                        match state {
                            ElementState::Pressed => match button {
                                MouseButton::Left if modifiers_state.ctrl() => camera_inputs.mouse_middle_click = true,
                                MouseButton::Left => camera_inputs.mouse_left_click = true,
                                MouseButton::Middle => camera_inputs.mouse_middle_click = true,
                                _ => {}
                            },
                            ElementState::Released => match button {
                                // we don't know if we started with the ctrl button enabled,
                                // which means we don't know if we are rotating or dragging.
                                // therefore just disable both of them.
                                MouseButton::Left => {
                                    camera_inputs.mouse_left_click = false;
                                    camera_inputs.mouse_middle_click = false;
                                },
                                MouseButton::Middle => camera_inputs.mouse_middle_click = false,
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    });
}
