extern crate pest;
#[macro_use]
extern crate pest_derive;
use clap::{Parser, ValueEnum};

use egui_wgpu::renderer::{ScreenDescriptor, Renderer};
use egui::FontId;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
};

//use imgui::{FontSource, FontGlyphRanges};

mod util;
mod rendering;
mod state;
mod device_manager;
mod shader_processing;
mod node_graph;
mod gui;
mod file_io;
mod parser;
mod compute_graph;

use std::{env, time::Instant, path::PathBuf};

use crate::{state::{Action, AppState, UserState}, util::files_from_names};
use egui_winit::{State};

#[derive(Debug)]
pub enum CustomEvent {
    NewFile,
    ShowOpenDialog,
    OpenFile(std::path::PathBuf),
    SaveFile(std::path::PathBuf),
    ExportGraphPng(std::path::PathBuf),
    ExportScenePng(std::path::PathBuf),
    RequestExit,
    RequestRedraw,
    ProcessUserState,
    MouseFreeze,
    MouseThaw,
}

///// This is the repaint signal type that egui needs for requesting a repaint from another thread.
///// It sends the custom RequestRedraw event to the winit event loop.
//struct ExampleRepaintSignal(std::sync::Mutex<winit::event_loop::EventLoopProxy<CustomEvent>>);
//
//impl epi::backend::RepaintSignal for ExampleRepaintSignal {
//    fn request_repaint(&self) {
//        self.0.lock().unwrap().send_event(CustomEvent::RequestRedraw).ok();
//    }
//}

#[derive(ValueEnum, Clone, Debug)]
enum GuiSelect {
    Ferre,
    Nodes,
}

#[derive(Parser)]
#[command(name = "FranzPlot")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Optional file to open at launch
    filename: Option<PathBuf>,

    /// Do not open the UI, export PNG of the rendered scene to this file
    #[arg(long, value_name = "EXPORT_SCENE")]
    export_scene: Option<PathBuf>,

    /// Do not open the UI, export PNG of the node graph to this file
    #[arg(long, value_name = "EXPORT_GRAPH")]
    export_graph: Option<PathBuf>,

    /// Choose a different wgpu backend from the default one. Options are:
    /// "vulkan", "metal", "dx12", "dx11", "gl"
    #[arg(short, long, value_name = "WGPU_BACKEND")]
    backend: Option<String>,

    /// Choose the GUI you want to use
    #[arg(short, long, value_enum, value_name = "GUI")]
    gui: Option<GuiSelect>,

    /// Sets a tracing folder for debug purposes
    #[arg(short, long, value_name = "TRACING")]
    tracing: Option<PathBuf>,

    /// Pauses execution before backend initialization; each 'p' adds 5 seconds
    #[arg(short, action = clap::ArgAction::Count)]
    pause: u8,
}


fn main() -> Result<(), String>{
    let cli = Cli::parse();
    //wgpu_subscriber::initialize_default_subscriber(None);

    use std::{thread, time};
    let seconds_to_wait = 5 * cli.pause as u64;
    thread::sleep(time::Duration::from_secs(seconds_to_wait));

    env_logger::init();
    let opt_input_file = cli.filename.as_deref();

    let opt_export_scene = cli.export_scene.as_deref();
    let opt_export_graph = cli.export_graph.as_deref();

    let opt_backend = cli.backend.as_deref().map(|name| {
        match name.to_lowercase().as_str()  {
            "vulkan" => wgpu::Backends::VULKAN,
            "metal" => wgpu::Backends::METAL,
            "dx12" => wgpu::Backends::DX12,
            "dx11" => wgpu::Backends::DX11,
            "gl" => wgpu::Backends::GL,
            "webgpu" => wgpu::Backends::BROWSER_WEBGPU,
            other => panic!("Unknown backend: {}", other),
        }
    });

    let selected_gui = cli.gui.unwrap_or(GuiSelect::Ferre);

    let device_manager = device_manager::Manager::new(cli.tracing.as_deref(), opt_backend);
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
            let error_message = "Could not find the 'resources' folder. Make sure to extract \
                all the contents of the compressed folder, and to keep the Franzplot executable \
                next to the 'resources' folder";
            rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Error)
                .set_description(error_message)
                .set_buttons(rfd::MessageButtons::Ok)
                .show();

            return Err(error_message.to_string());
        }
    };
    let mut materials_dir = resources_path.clone();
    materials_dir.push("materials");

    // load all masks that will be available in the rendering node and make them avail in egui
    let masks_dir = resources_path.join("masks");
    let mask_names: [&str; 5] = [
        "checker_8.png",
        "h_stripes_16.png",
        "v_stripes_16.png",
        "blank.png",
        "alpha_grid.png",
    ];
    let mask_files = util::files_from_names(&masks_dir, mask_names);
    let masks = util::load_textures_to_wgpu(&device_manager, &mask_files);

    // do the same for materials.
    let materials_dir = resources_path.join("materials");
    let material_names: [&str; 8] = [
        "00_blue.png",
        "01_green.png",
        "02_yellow.png",
        "03_orange.png",
        "04_purple.png",
        "05_pink.png",
        "06_cyan.png",
        "07_white.png",
    ];
    let material_files = util::files_from_names(&materials_dir, material_names);
    let materials = util::load_textures_to_wgpu(&device_manager, &material_files);

    // load all the models
    let models_dir = resources_path.join("models");
    let model_names: [&str; 6] = [
        "cone.obj",
        "cube.obj",
        "cylinder.obj",
        "dice.obj",
        "pyramid.obj",
        "sphere.obj",
    ];
    let model_files = util::files_from_names(&models_dir, model_names);
    let models = util::load_models_to_wgpu(&device_manager.device, &model_files);
    // the model_labels is an extra that might be used by the GUI to show names for the models
    let model_labels = model_names.map(|name| {
        name.trim_end_matches(".obj")
    });

    // We can finally put all the materials, models and masks inside an Asset struct
    let assets = state::Assets {
        materials,
        models,
        masks,
    };


    // UP TO THIS POINT, initialization is exactly the same.
    // now, we need to do something different if we are running headless
    // or we are running with a GUI
    let export_only = opt_export_graph.is_some() || opt_export_scene.is_some();
    if export_only {
        // headless mode!
        // First, create an AppState + UserState (not the full State, which implies a running winit GUI)
        let mut app_state = AppState::new(device_manager, assets);
        let input_file = opt_input_file.expect("An input file is required when exporting a scene or a graph!");
        let file_contents = file_io::File::read_from_frzp(input_file)?;//.expect("Error opening the input file");
        // load the user_state but ignore ferre_data
        let file_io::VersionV2::V20{ mut user_state, .. } = file_contents.convert_to_v2()?;
        if let Some(scene_path) = opt_export_scene {
            app_state.camera.set_x1_y1_z1_wide(); // export scene with dafault wide angle view
            util::create_scene_png(&mut app_state, &mut user_state, &scene_path);
        }
        if let Some(graph_path) = opt_export_graph {
            util::create_graph_png(&mut app_state, &user_state, &graph_path);
        }
        // after exporting, just exit
        return Ok(());
    }

    // if we are NOT running in headless mode, then we need to create the event loop and initialize
    // the window

    let mut old_instant = std::time::Instant::now();
    let mut modifiers_state = winit::event::ModifiersState::default();

    let event_loop = EventLoopBuilder::<CustomEvent>::with_user_event().build();
    let event_loop_proxy = event_loop.create_proxy();
    let executor = util::Executor::new();

    let window_size = if let Some(monitor) = event_loop.primary_monitor() {
        // web winit always reports a size of zero
        #[cfg(not(target_arch = "wasm32"))]
        let screen_size = monitor.size();
        winit::dpi::PhysicalSize::new(screen_size.width * 3 / 4, screen_size.height * 3 / 4)
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
        .with_title("Franzplot")
        .with_window_icon(Some(icon))
        .with_inner_size(window_size);
    let window = builder.build(&event_loop).unwrap();

    // We use the egui_wgpu_backend crate as the render backend.
    let mut egui_rpass = egui_wgpu::Renderer::new(&device_manager.device, crate::rendering::SWAPCHAIN_FORMAT, None, 1); // TODO: investigate more how to properly set this

    // last, initialize the gui and the state with the available assets.
    let mask_ids: Vec<egui::TextureId> = assets.masks.iter()
        .map(|elem| egui_rpass.register_native_texture(&device_manager.device, &elem.view, egui_wgpu::wgpu::FilterMode::Linear))
        .collect();

    let material_ids: Vec<egui::TextureId> = assets.materials.iter()
        .map(|elem| egui_rpass.register_native_texture(&device_manager.device, &elem.view, egui_wgpu::wgpu::FilterMode::Linear))
        .collect();

    let availables = gui::Availables {
        mask_ids,
        material_ids,
        model_names: model_labels.to_vec(),
    };
    let gui: Box<dyn gui::Gui> = match selected_gui {
        GuiSelect::Ferre => Box::new(gui::FerreGui::new(event_loop_proxy.clone())),
        GuiSelect::Nodes => Box::new(gui::NodeGui::new(event_loop_proxy.clone(), availables)),
    };

    let mut state = state::State::new(AppState::new(device_manager, assets), egui_rpass, gui, &window, &event_loop);
    if let Some(file) = opt_input_file {
        let action = Action::OpenFile(file);
        state.process(action)?;
    } else {
        println!("FranzPlot starting, no file selected.");
    }

    let hidpi_factor = window.scale_factor();
    window.set_min_inner_size(Some(winit::dpi::LogicalSize::new(200.0, 100.0)));

    let mut camera_inputs = rendering::camera::InputState::default();
    let mut cursor_position = winit::dpi::PhysicalPosition::<i32>::new(0, 0);
    let mut mouse_frozen = false;


    let start_time = Instant::now();
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
                //imgui.io_mut().update_delta_time(frame_duration); // this function only computes imgui internal time delta
                old_instant = now;
            }
            Event::Suspended => {
            }
            Event::Resumed => {
            }
            Event::DeviceEvent { .. } => {
            }
            // Emitted when all of the event loop's input events have been processed and redraw processing is about to begin.
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::WindowEvent { event: WindowEvent::Resized(physical_size), .. } => {
                state.resize_frame(physical_size);
            }
            // TODO: this might be useful for the web or mobile version
            //Event::WindowEvent { event: WindowEvent::ScaleFactorChanged { new_inner_size, .. }, ..} => {
            //    // new_inner_size is &&mut so we have to dereference it twice
            //    state.resize_frame(*new_inner_size);
            //}
            // Begin rendering. During each iteration of the event loop, Winit will aggregate duplicate redraw requests
            // into a single event, to help avoid duplicating rendering work.
            Event::RedrawRequested(_window_id) => {
                state.render_frame(&window).expect("rendering of the main UI failed");
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
                    CustomEvent::RequestRedraw => {
                        window.request_redraw();
                    },
                    CustomEvent::ProcessUserState => {
                        state.user_to_app_state();
                    },
                    CustomEvent::ShowOpenDialog => {
                        file_io::async_pick_open(event_loop_proxy.clone(), &executor);
                    },
                    CustomEvent::NewFile => {
                        // when we are actually creating a new file, we need to both
                        // "reset the state" and "reset the gui"
                        // state reset is done with the appropriate action
                        let action = Action::NewFile();
                        state.process(action).expect("failed to create a new file");
                        // and after that we can tell the GUI to also reset some of its parts
                        //rust_gui.reset_undo_history(&state);
                        //rust_gui.reset_nongraph_data();
                    },
                    CustomEvent::RequestExit => {
                        *control_flow = ControlFlow::Exit;
                    },
                    CustomEvent::SaveFile(path_buf) => {
                        let action = Action::WriteToFile(&path_buf);
                        match state.process(action) {
                            Ok(()) => {
                            },
                            Err(error) => {
                                file_io::async_dialog_failure(&executor, error);
                            }
                        }
                        //rust_gui.graph_edited = false;
                    },
                    CustomEvent::OpenFile(path_buf) => {
                        let action = Action::OpenFile(&path_buf);
                        match state.process(action) {
                            Ok(()) => {
                                //rust_gui.reset_undo_history(&state);
                                //rust_gui.reset_nongraph_data();
                                //rust_gui.opened_tab[0] = true;
                            },
                            Err(error) => {
                                file_io::async_dialog_failure(&executor, error);
                            }
                        }
                    },
                    CustomEvent::ExportGraphPng(path_buf) => {
                        //println!("Exporting graph: {:?}", &path_buf);
                        //// zoom out once or twice
                        //state.user.node_graph.zoom_down_graph([0.0, 0.0]);
                        ////state.user.graph.zoom_down_graph([0.0, 0.0]);
                        //state.user.node_graph.push_all_to_corner();
                        //state.user.node_graph.push_positions_to_imnodes();
                        //util::create_graph_png(&mut state, &path_buf,&window,&mut platform,&mut renderer,&mut rust_gui,&mut imgui, window_size.to_logical(hidpi_factor));
                    },
                    CustomEvent::ExportScenePng(path_buf) => {
                        println!("Exporting scene: {:?}", &path_buf);
                        util::create_scene_png(&mut state.app, &mut state.user, &path_buf);
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
            }
            // match a very specific WindowEvent: user-requested closing of the application
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                //if rust_gui.graph_edited {
                //    file_io::async_confirm_exit(event_loop_proxy.clone(), &executor)
                //} else {
                    *control_flow = ControlFlow::Exit;
                //}
            }
            Event::WindowEvent { event: WindowEvent::DroppedFile(file_path), .. } => {
                //if rust_gui.graph_edited {
                //    file_io::async_confirm_load(event_loop_proxy.clone(), &executor, file_path);
                //} else {
                event_loop_proxy.send_event(CustomEvent::OpenFile(file_path)).unwrap();
                //}
            },
            Event::WindowEvent { event, .. } => {
                let _event_response = state.egui_state.on_event(&state.egui_ctx, &event);
            },
        }
    });
}
