use std::collections::BTreeMap;

use egui::TextureId;
use serde::{Serialize, Deserialize};

use crate::CustomEvent;
use crate::compute_graph::globals::NameValuePair;
use crate::node_graph::NodeID;
use crate::{util, file_io};
use crate::state::{UserState, AppState, user_to_app_state};

#[derive(Deserialize, Serialize)]
pub struct Explanation {
}

#[derive(Deserialize, Serialize)]
#[derive(Clone, Default)]
pub struct FerreData {
    steps: BTreeMap<NodeID, (bool, String)>,
}

pub struct FerreGui {
    scene_extent: wgpu::Extent3d,
    winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>,
    ferre_data: FerreData,
    executor: util::Executor,
}

impl FerreGui {
    pub fn new(winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>) -> Self {
        FerreGui {
            ferre_data: Default::default(),
            scene_extent: wgpu::Extent3d::default(),
            winit_proxy,
            executor: util::Executor::new(),
        }
    }
}

impl super::Gui for FerreGui {
    fn show(&mut self, ctx: &egui::Context, app_state: &mut AppState, user_state: &mut UserState, texture_id: TextureId) {

        egui::SidePanel::left("procedure panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open file").clicked() {
                    file_io::async_pick_open(self.winit_proxy.clone(), &self.executor);
                }
                if ui.button("Save file").clicked() {
                    file_io::async_pick_save(self.winit_proxy.clone(), &self.executor);
                }
                if ui.button("Add test entry").clicked() {
       //             self.ferre_data.steps.insert(2, "questo è un segmento".to_string());
                }
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .show(ui, |ui| {
                    for (id, comment) in self.ferre_data.steps.iter_mut() {
                        ui.horizontal(|ui| {
                            let maybe_node_title = user_state.node_graph.get_node(*id);
                            let title = if let Some(node) = maybe_node_title {
                                node.title.clone()
                            } else {
                                "unknown node".to_string()
                            };
                            ui.vertical(|ui| {
                                let node_label = format!("Node {}: {}", id, title);
                                ui.label(node_label);
                                ui.small(&comment.1);
                            });

                            let widget_id = egui::Id::new(id);
                            if ui.button("show").clicked() {
                                if !comment.0 {
                                    user_to_app_state(app_state, user_state, Some(vec![*id]));
                                }
                                comment.0 = !comment.0;
                            }
                            let animation_status = ctx.animate_bool_with_time(widget_id, comment.0, 2.0);
                            app_state.update_globals(vec![
                                NameValuePair {
                                    name: "a".into(),
                                    value: animation_status,
                                }
                            ]);
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
    }

    /// Ask the UI what size the 3D scene should be. This function gets called after show(), but
    /// before the actual rendering happens.
    fn compute_scene_size(&self) -> Option<wgpu::Extent3d> {
        Some(self.scene_extent)
    }

    /// handle loading of the ferre data
    fn load_ferre_data(&mut self, ferre_data: FerreData) {
        self.ferre_data = ferre_data;
    }

    fn export_ferre_data(&self) -> Option<FerreData> {
        Some(self.ferre_data.clone())
    }
}
