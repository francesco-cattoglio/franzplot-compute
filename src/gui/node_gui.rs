use egui::TextureId;

use crate::CustomEvent;
use crate::node_graph::{Node, NodeContents};
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
    style: egui::style::Style,
    editor_offset: egui::Vec2,
    top_area_h: f32,
    left_area_w: f32,
    scene_extent: wgpu::Extent3d,
    winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>,
}

impl NodeGui {
    pub fn new(winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>) -> Self {
        NodeGui {
            current_tab: GuiTab::Graph,
            top_area_h: 0.0,
            style: Default::default(),
            editor_offset: egui::vec2(0.0, 0.0),
            left_area_w: 0.0,
            ferre_data: None,
            scene_extent: wgpu::Extent3d::default(),
            winit_proxy,
        }
    }

    fn show_top_bar(&mut self, ctx: &egui::Context) -> egui::Rect {
        let inner = egui::Area::new("top menu area")
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(0.0, 0.0))
            .show(ctx, |ui| {
                // Add a frame that looks like a window but has no rounding in the corners!
                egui::Frame::window(&self.style)
                    .rounding(egui::Rounding::none())
                    .show(ui, |ui| {
                        ui.set_min_width(ui.max_rect().width());
                        ui.horizontal(|ui| {
                            ui.label("Menus will go here");
                            ui.separator();
                            ui.selectable_value(&mut self.current_tab, GuiTab::Graph, "Graph");
                            ui.selectable_value(&mut self.current_tab, GuiTab::Scene, "Scene");
                            ui.selectable_value(&mut self.current_tab, GuiTab::Settings, "Settings");
                        });
                    });
                });
        inner.response.rect
    }

    fn show_graph(&mut self, ctx: &egui::Context, avail_rect: egui::Rect, app_state: &mut AppState, user_state: &mut UserState) {
        let inner = egui::Area::new("global vars area")
            .order(egui::Order::Foreground)
            .fixed_pos(avail_rect.min)
            .show(ctx, |ui| {
                // Add a frame that looks like a window but has no rounding in the corners!
                egui::Frame::window(&self.style)
                    .rounding(egui::Rounding::none())
                    .show(ui, |ui| {
                        ui.set_min_height(avail_rect.height());
                        ui.set_max_width(128.0);
                        ui.vertical(|ui| {
                            ui.label("Global vars will go here");
                            ui.separator();
                            ui.label("Global vars will go here");
                            ui.separator();
                            ui.label("Global vars will go here");
                            ui.separator();
                            ui.label("Global vars will go here");
                            ui.separator();
                            ui.label("Global vars will go here");
                        });
                    });
                });
        let used_x = inner.response.rect.width();

        for (node_id, node) in user_state.node_graph.get_nodes_mut() {
            let Node {
                title,
                position,
                error,
                contents
            } = node;
            let window_response = egui::Window::new(title.clone())
                .id(egui::Id::new(node_id))
                .current_pos(egui::pos2(position[0] + self.editor_offset.x, position[1] + self.editor_offset.y))
                .drag_bounds(egui::Rect::EVERYTHING)
                .show(ctx, |ui| {
                    self.add_node_contents(ui, contents);
                    ui.label("This will contain the node attributes");
                });
        }
        //egui::SidePanel::left("globals edit").show(ctx, |ui| {
        //    if ui.button("render scene from graph").clicked() {
        //        let result = user_to_app_state(app_state, user_state);
        //        if result.is_ok() {
        //            self.current_tab = GuiTab::Scene;
        //        }
        //    }
        //});
        egui::CentralPanel::default().show(ctx, |ui| {
            let (id, rect) = ui.allocate_space(ui.available_size());
            let response = ui.interact(rect, id, egui::Sense::click_and_drag());
            if response.dragged_by(egui::PointerButton::Middle) {
                self.editor_offset += response.drag_delta();
            }
        }); // central panel
    }

    fn add_node_contents(&mut self, ui: &mut egui::Ui, contents: &mut NodeContents) {
        ui.label("will add");
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
        let used_rect = self.show_top_bar(ctx);
        let avail_rect = egui::Rect {
            min: egui::pos2(used_rect.min.x, used_rect.max.y),
            max: ctx.available_rect().max
        };
        match self.current_tab {
            GuiTab::Graph => self.show_graph(ctx, avail_rect, app_state, user_state),
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
