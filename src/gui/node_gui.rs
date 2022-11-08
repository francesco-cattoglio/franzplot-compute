use egui::TextureId;

use crate::CustomEvent;
use crate::state::{UserState, AppState, user_to_app_state};

use super::FerreData;

#[derive(PartialEq)]
enum GuiTab {
    Graph,
    Scene,
    Settings,
}

pub struct NodeGui {
    ferre_data: Option<FerreData>,
    current_tab: GuiTab,
    scene_extent: wgpu::Extent3d,
    winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>,
}

impl NodeGui {
    pub fn new(winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>) -> Self {
        NodeGui {
            current_tab: GuiTab::Graph,
            ferre_data: None,
            scene_extent: wgpu::Extent3d::default(),
            winit_proxy,
        }
    }

    fn show_graph(&mut self, ctx: &egui::Context, app_state: &mut AppState, user_state: &mut UserState) {
        egui::SidePanel::left("globals edit").show(ctx, |ui| {
            if ui.button("render scene from graph").clicked() {
                let result = user_to_app_state(app_state, user_state);
                if result.is_ok() {
                    self.current_tab = GuiTab::Scene;
                }
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Node graph should go here");
        }); // central panel
    }

    fn show_scene(&mut self, ctx: &egui::Context, app_state: &mut AppState, texture_id: TextureId) {
        egui::SidePanel::left("globals live").show(ctx, |ui| {
            ui.label("here be variables with sliders");
        });
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
    }

    fn show_settings(&mut self, ctx: &egui::Context, app_state: &mut AppState) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Scene settings shall go here");
        }); // central panel
    }
}

impl super::Gui for NodeGui {
    fn show(&mut self, ctx: &egui::Context, app_state: &mut AppState, user_state: &mut UserState, texture_id: TextureId) {
        egui::TopBottomPanel::top("file panel").show(ctx, |ui| {
            ui.horizontal(|ui|{
                ui.label("Menus will go here");
                ui.separator();
                ui.selectable_value(&mut self.current_tab, GuiTab::Graph, "Graph");
                ui.selectable_value(&mut self.current_tab, GuiTab::Scene, "Scene");
                ui.selectable_value(&mut self.current_tab, GuiTab::Settings, "Settings");
            });

        });

        match self.current_tab {
            GuiTab::Graph => self.show_graph(ctx, app_state, user_state),
            GuiTab::Scene => self.show_scene(ctx, app_state, texture_id),
            GuiTab::Settings => self.show_settings(ctx, app_state),
        }

    }

    fn load_ferre_data(&mut self, ferre_data: FerreData) {
        self.ferre_data = Some(ferre_data);
    }

    fn export_ferre_data(&self) -> Option<FerreData> {
        self.ferre_data.clone()
    }

    fn compute_scene_size(&self) -> Option<wgpu::Extent3d> {
        if self.current_tab == GuiTab::Scene {
            Some(self.scene_extent)
        } else {
            None
        }
    }

}
