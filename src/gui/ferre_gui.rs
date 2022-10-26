use egui::TextureId;

use crate::CustomEvent;
use crate::state::{UserState, AppState};

pub struct FerreGui {
    scene_extent: wgpu::Extent3d,
    winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>,
}

impl FerreGui {
    pub fn new(winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>) -> Self {
        FerreGui {
            scene_extent: wgpu::Extent3d::default(),
            winit_proxy,
        }
    }
}

impl super::Gui for FerreGui {
    fn show(&mut self, raw_input: egui::RawInput, app_state: &mut AppState, user_state: &mut UserState, texture_id: TextureId) -> egui::FullOutput {
        let ctx = &app_state.egui_ctx;
        ctx.begin_frame(raw_input);

        egui::SidePanel::left("procedure panel").show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .show(ui, |ui| {
                    for (id, node) in user_state.node_graph.get_nodes() {
                        ui.horizontal(|ui| {
                            let node_label = format!("Node {}: {}", id, node.title);
                            ui.label(node_label);
                            if ui.button("test").clicked() {
                                let result = self.winit_proxy.send_event(CustomEvent::ProcessUserState);
                                if let Ok(()) = result {
dbg!("good!");
                                } else {
dbg!("bad!");
                                }
                            }
                        }); // horizontal

                    }
                }); // Scrollable area.
        }); // left panel
        egui::TopBottomPanel::bottom("variables panel").show(ctx, |ui| {
            let globals = &user_state.globals;
            for variable_name in &globals.names {
                ui.label(variable_name);
            }
        }); // bottom panel
        egui::CentralPanel::default().show(ctx, |ui| {
            // compute avail size
            let avail = ui.available_size();
            // store this size so that we can report it properly to the State on next frame
            self.scene_extent = wgpu::Extent3d {
                width: (avail.x * ctx.pixels_per_point()) as u32,
                height: (avail.y * ctx.pixels_per_point()) as u32,
                depth_or_array_layers: 1,
            };
            ui.image(texture_id, avail);
        }); // central panel

//let texture_size = wgpu::Extent3d {
//    width: 320,
//    height: 320,
//    ..Default::default()
//};
//let render_request = Action::RenderScene(texture_size, &scene_view);
//state.process(render_request).expect("failed to render the scene due to an unknown error");
        // End the UI frame. Returning the output that will be used to draw the UI on the backend.
        ctx.end_frame()
    }

    /// Ask the UI what size the 3D scene should be. This function gets called after show(), but
    /// before the actual rendering happens.
    fn compute_scene_size(&self) -> Option<wgpu::Extent3d> {
        Some(self.scene_extent)
    }
}
