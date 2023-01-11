use std::io::BufReader;
use std::path::PathBuf;

use crate::CustomEvent;
use crate::compute_graph::ComputeGraph;
use crate::compute_graph::globals::NameValuePair;
use crate::device_manager::Manager;
use crate::file_io::File;
use crate::file_io::VersionV2;
use crate::gui::Gui;
use crate::rendering::SWAPCHAIN_FORMAT;
use crate::rendering::camera;
use crate::rendering::SceneRenderer;
use crate::rendering::model::Model;
use crate::rendering::texture::Texture;

pub mod action;
pub use action::Action;
pub mod user_state;
pub use user_state::UserState;
use winit::dpi::PhysicalSize;

// The State struct encapsulates the whole application state,
// the GUI takes a mutable reference to the state and modifies it
// according to user input. The state contains both the data
// that the user is constantly editing (UserState) and the "rendered result"
// of that data (AppState). This distinction is very important w.r.t
// saving to file: we don't want to serialize compute shaders,
// we only want to save the graph, the variables and the scene settings.

pub struct Assets {
    pub materials: Vec<Texture>,
    pub masks: Vec<Texture>,
    pub models: Vec<Model>,
}

pub struct Sensitivity {
    pub graph_zoom: f32,
    pub scene_zoom: f32,
    pub camera_horizontal: f32,
    pub camera_vertical: f32,
}

impl Default for Sensitivity {
    fn default() -> Self {
        Sensitivity {
            graph_zoom: 1.0,
            scene_zoom: 1.0,
            camera_horizontal: 1.0,
            camera_vertical: 1.0,
        }
    }
}

pub struct AppState {
    pub camera_controller: Box<dyn camera::Controller>,
    pub camera_enabled: bool,
    pub camera_lock_up: bool,
    pub camera_ortho: bool, // TODO: all these camera settings should NOT be here, move them somewhere else!
    pub auto_scene_on_processing: bool,
    pub camera: camera::Camera,
    pub assets: Assets,
    pub manager: Manager,
    pub comp_graph: Option<ComputeGraph>,
    pub renderer: SceneRenderer,
    pub sensitivity: Sensitivity,
    pub parts_list: Option<Vec<(String, PathBuf)>>,
}

impl AppState {
    pub fn new(manager: Manager, assets: Assets) -> Self {
        let camera = camera::Camera::default();
        let camera_controller = Box::new(camera::VTKController::new());
        AppState {
            //computable_scene,
            assets,
            camera,
            auto_scene_on_processing: true,
            camera_enabled: false,
            camera_lock_up: true,
            camera_ortho: false,
            camera_controller,
            renderer: SceneRenderer::new_with_axes(&manager),
            manager,
            comp_graph: None,
            parts_list: None,
            sensitivity: Sensitivity::default(),
        }
    }
    pub fn set_wireframe_axes(&mut self, length: i32, cross_size: f32) {
        self.renderer.set_wireframe_axes(&self.manager, length, cross_size);
    }

    pub fn clear_wireframe_axes(&mut self) {
        self.renderer.clear_wireframe_axes();
    }

    pub fn set_axes_labels(&mut self, axis_length: i32, label_size: f32) {
        self.renderer.set_axes_labels(&self.manager, axis_length as f32, label_size);
    }

    pub fn clear_axes_labels(&mut self) {
        self.renderer.clear_axes_labels();
    }

    pub fn update_camera(&mut self, camera_inputs: &camera::InputState) {
        if self.camera_enabled {
            self.camera_controller.update_camera(&mut self.camera, camera_inputs, &self.sensitivity, self.camera_lock_up);
        }
    }

    pub fn update_globals(&mut self, pairs: Vec<NameValuePair>) {
        if let Some(graph) = &mut self.comp_graph {
            graph.update_globals(&self.manager.device, &self.manager.queue, pairs);
        }
    }

    pub fn read_globals(&self) -> Vec<NameValuePair> {
        if let Some(graph) = &self.comp_graph {
            graph.get_globals().clone()
        } else {
            Vec::new()
        }
    }

    pub fn render_scene(&mut self, extent: wgpu::Extent3d, view: &wgpu::TextureView) -> Result<(), String> {
        // create aliases
        let renderer = &mut self.renderer;
        let camera = &mut self.camera;
        renderer.resize_if_needed(&self.manager, extent);
        let aspect_ratio = extent.width as f32/extent.height as f32;
        let projection_matrix = if self.camera_ortho {
            camera.build_ortho_matrix(aspect_ratio)
        } else {
            camera.build_projection_matrix(aspect_ratio)
        };
        renderer.update_proj(projection_matrix);
        renderer.update_view(camera.build_view_matrix());
        // after updating everything, redraw the scene to the texture
        renderer.render(&self.manager, view);
        Ok(())
    }
}


// TODO: RENAME THIS, maybe even move it somewhere else
pub fn user_to_app_state(app: &mut AppState, user: &mut UserState) -> Result<(), String> {
    // - clear previous node graph errors
    // - try to create a new compute graph
    // - if successful, update the scene rendering and report recoverable errors
    // - if unsuccessful, report the unrecoverable error to the user
    let process_result = crate::compute_graph::create_compute_graph(&app.manager.device, &app.assets, user);
    match process_result {
        Ok((compute_graph, recoverable_errors)) => {
            // run the first compute, and create the matcaps in the SceneRenderer
            compute_graph.run_compute(&app.manager.device, &app.manager.queue);
            app.renderer.recreate_matcaps(&app.manager, &app.assets, compute_graph.all_matcaps());
            app.comp_graph = Some(compute_graph);
            if recoverable_errors.is_empty() {
                Ok(())
            } else {
                for error in recoverable_errors.into_iter() {
                    user.node_graph.mark_error(error.into());
                }
                Err("Recoverable errors detected".into())
            }
        },
        Err(unrecoverable_error) => {
            let formatted_error = format!("Unrecoverable error: {:?}", &unrecoverable_error);
            user.node_graph.mark_error(unrecoverable_error.into());
            Err(formatted_error) // TODO: better handling
        }
    }
}

pub struct State {
    pub app: AppState,
    pub user: UserState,
    pub gui: Box<dyn Gui>,
    pub event_loop: winit::event_loop::EventLoopProxy<CustomEvent>,
    pub egui_state: egui_winit::State,
    pub egui_ctx: egui::Context,
    pub egui_rpass: egui_wgpu::Renderer,
    pub screen_surface: wgpu::Surface,
    // BEWARE: we NEED to store the surface_config, because after a resize the window.inner_size()
    // does not reflect the correct value to be used by WGPU scissor rect, so we need to use
    // the width and height stored in the surface_config and use those at redering time
    pub surface_config: wgpu::SurfaceConfiguration,
    pub scene_texture: crate::rendering::texture::Texture,
    pub scene_texture_id: egui::TextureId,
    pub scene_extent: wgpu::Extent3d,
}

impl State {
    // this function will likely be called only once, at program start
    // at program start, we can just set the user and app data to its default value
    pub fn new(app: AppState, mut egui_rpass: egui_wgpu::Renderer, gui: Box<dyn Gui>, window: &winit::window::Window, event_loop: &winit::event_loop::EventLoop<CustomEvent>) -> Self {
        let mut egui_state = egui_winit::State::new(event_loop);
        egui_state.set_pixels_per_point(window.scale_factor() as f32);

        let (size, screen_surface) = unsafe {
            let size = window.inner_size();
            let surface = app.manager.instance.create_surface(window);
            (size, surface)
        };
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: SWAPCHAIN_FORMAT,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
        };
        screen_surface.configure(&app.manager.device, &surface_config);

        // first, create a texture that will be used to render the scene and display it inside of imgui
        let scene_extent = wgpu::Extent3d {
            width: 320,
            height:320,
            depth_or_array_layers: 1,
        };
        let scene_texture = Texture::create_output_texture(&app.manager.device, scene_extent, 1);
        let scene_texture_id = egui_rpass.register_native_texture(&app.manager.device, &scene_texture.view, egui_wgpu::wgpu::FilterMode::Linear);

        Self {
            app,
            user: UserState::default(),
            gui,
            egui_rpass,
            egui_state,
            egui_ctx: egui::Context::default(),
            event_loop: event_loop.create_proxy(),
            scene_texture_id,
            screen_surface,
            scene_texture,
            scene_extent,
            surface_config,
        }
    }

    pub fn resize_frame(&mut self, size: PhysicalSize<u32>) {
        let width = size.width;
        let height = size.height;
        if width >= 8 && height >= 8 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.screen_surface.configure(&self.app.manager.device, &self.surface_config);
        }
    }

    pub fn get_frame(&mut self) -> Option<wgpu::SurfaceTexture> {
        // get the framebuffer frame. We might need to re-create the swapchain if for some
        // reason our current one is outdated
        let maybe_frame = self.screen_surface.get_current_texture();
        match maybe_frame {
                Ok(surface_frame) => {
                    Some(surface_frame)
                }
                Err(wgpu::SurfaceError::Outdated) => {
                    // This interesting thing happens when we just resized the window but due to a
                    // race condition the winit ResizeEvent has not fired just yet. We might resize
                    // the swapchain here, but doing so would leave the app in a borked state:
                    // egui needs to be notified about the resize as well, otherwise it will run
                    // a scissor test on a framebuffer of a different physical size and the
                    // validation layer will panic. The best course of action is doing nothing at
                    // all, the problem will fix itself on the next frame, when the Resized event
                    // fires.
                    dbg!("outdated");
                    None
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    panic!("Out Of Memory error in frame rendering");
                }
                Err(wgpu::SurfaceError::Timeout) => {
                    println!("Warning: timeout error in frame rendering!");
                    None
                }
                Err(wgpu::SurfaceError::Lost) => {
                    println!("Warning: frame Lost error in frame rendering");
                    None
                }
        }
    }

    pub fn render_frame(&mut self, window: &winit::window::Window) -> Result<(), String> {
        // handle any action requested by the GUI
        // workaround for some messy ownership rules
        let mut workaround_action: Option<Action> = None;


        let maybe_extent = self.gui.compute_scene_size();
        if let Some(extent) = maybe_extent {
            // on the previous frame, the UI asked us to show the scene.
            // Compare the stored extent and decide if we need to re-create and register a wgpu
            // texture to accomodate for it.
            if self.scene_extent.ne(&extent)
                && extent.width >= 8 && extent.height >= 8 {
                    self.scene_extent = extent;
                    //let old_texture
                    self.scene_texture = Texture::create_output_texture(&self.app.manager.device, self.scene_extent, 1);
                    self.scene_texture_id = self.egui_rpass.register_native_texture(&self.app.manager.device, &self.scene_texture.view, egui_wgpu::wgpu::FilterMode::Linear);
            }
        }
        let raw_input = self.egui_state.take_egui_input(window);
        // begin frame
        self.egui_ctx.begin_frame(raw_input);
        // internally this will show all the UI elements
        {
            let maybe_action = self.gui.show(&self.egui_ctx, &mut self.app, &mut self.user, self.scene_texture_id);
            match maybe_action {
                Some(Action::OpenFile(path_buf)) => {
                    workaround_action = Some(Action::OpenFile(path_buf));
                }
                Some(Action::OpenPart(path_buf)) => {
                    workaround_action = Some(Action::OpenPart(path_buf));
                }
                _ => {}
            }
        }
        // End the UI frame. Returning the output that will be used to draw the UI on the backend.
        let full_output = self.egui_ctx.end_frame();

        // The actual rendering of the scene is done AFTER the UI code run, so any kind of change
        // to the global vars or the camera shows immediately
        if maybe_extent.is_some() {
            self.app.render_scene(self.scene_extent, &self.scene_texture.view);
        };

        // back to egui rendering: register all the changes, tessellate the shapes, etc
        self.egui_state.handle_platform_output(window, &self.egui_ctx, full_output.platform_output);
        let paint_jobs = self.egui_ctx.tessellate(full_output.shapes);
        // acquire next frame, or update the swapchain if a resize occurred
        let frame = if let Some(frame) = self.get_frame() {
            frame
        } else {
            // if we are unable to get a frame, skip rendering altogether
            return Ok(());
        };

        // declare an alias to make the rest of the code readable
        let manager = &self.app.manager;
        // use the acquired frame for a rendering pass, which will clear the screen and render the gui
        let mut encoder: wgpu::CommandEncoder =
            self.app.manager.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        // Upload all resources for the GPU.
        let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
            size_in_pixels: [self.surface_config.width, self.surface_config.height],
            pixels_per_point: window.scale_factor() as f32,
        };
        for (id, image_delta) in full_output.textures_delta.set {
            self.egui_rpass.update_texture(&manager.device, &manager.queue, id, &image_delta);
        }
        self.egui_rpass.update_buffers(
            &manager.device,
            &manager.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor
        );

        // Record all render passes.
        let frame_view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &frame_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
            label: Some("egui_render"),
        });
        self.egui_rpass
            .render(
                &mut render_pass,
                &paint_jobs,
                &screen_descriptor,
            );
        }
        // Submit the commands.
        manager.queue.submit(std::iter::once(encoder.finish()));

        // Redraw egui
        frame.present();

        if let Some(action) = workaround_action {
            self.process(action);
        }
        Ok(())
    }

    pub fn user_to_app_state(&mut self) -> Result<(), String> {
        user_to_app_state(&mut self.app, &mut self.user)
    }

    pub fn process(&mut self, action: Action) -> Result<(), String> {
        match action {
            Action::WriteToFile(path) => {
                File::V2(VersionV2::V20 {
                    user_state: self.user.clone(),
                    ferre_data: self.gui.export_ferre_data(),
                }).write_to_frzp(path)
            } ,
            //TODO: DRY, some code is copy-pasted for the OpenPart branch
            Action::OpenFile(path) => {
                // first attempt at reading the file as a json list of parts:
                let maybe_vec_parts = crate::file_io::load_file_part_list(&path)?;
                let frzp_file = if let Some(parts_list) = maybe_vec_parts {
                    // we succesfully open a json list of parts.
                    let path_buf = parts_list.first().ok_or_else(|| String::from("The JSON part list cannot be empty"))?.1.clone();
                    self.app.parts_list = Some(parts_list);
                    path_buf
                } else {
                    //self.app.parts_list = None;
                    path
                };
                let VersionV2::V20 { user_state, ferre_data } = File::read_from_frzp(&frzp_file)?.convert_to_v2()?;
                self.user = user_state;
                self.gui.mark_new_file_open(&self.egui_ctx);
                if let Some(ferre) = ferre_data {
                    self.gui.load_ferre_data(&self.egui_ctx, ferre);
                    // Quick hack: by default, always process the scene when we open
                    // something that could be used by the Ferre GUI
                    user_to_app_state(&mut self.app, &mut self.user);
                }
                Ok(())
            },
            Action::OpenPart(path) => {
                // Since we know this is a part, we can just load it as frzp file.
                let VersionV2::V20 { user_state, ferre_data } = File::read_from_frzp(&path)?.convert_to_v2()?;
                self.user = user_state;
                self.gui.mark_new_part_open(&self.egui_ctx);
                if let Some(ferre) = ferre_data {
                    self.gui.load_ferre_data(&self.egui_ctx, ferre);
                    // Quick hack: by default, always process the scene when we open
                    // something that could be used by the Ferre GUI
                    user_to_app_state(&mut self.app, &mut self.user);
                }
                Ok(())
            },
            Action::NewFile() => {
                // reset the user state: this will zero out the node graph and its global vars
                self.user = UserState::default();
                // clear all the created renderables and the entire compute graph
                self.app.renderer.clear_matcaps();
                self.app.comp_graph = None;
                // new timestamp for the new file
                Ok(())
            },
            Action::RenderScene(extent, view) => {
                // create aliases
                self.app.render_scene(extent, view)
            },
            Action::ProcessUserState() => {
                user_to_app_state(&mut self.app, &mut self.user)
            }
            Action::UpdateGlobals(pairs) => {
                // if the compute graph exists, tell it to update the globals
                if let Some(graph) = &mut self.app.comp_graph {
                    graph.update_globals(&self.app.manager.device, &self.app.manager.queue, pairs);
                    Ok(())
                } else {
                    dbg!("tried to update globals, but there is no graph!"); // TODO: better handling
                    Ok(())
                }
            }
            Action::RenderUI(window) => {
                self.render_frame(window)
            }
        }
    }
}
