use std::collections::HashMap;

use egui::TextureId;
use pest::unicode::UPPERCASE_LETTER;

use crate::CustomEvent;
use crate::node_graph::{Node, NodeContents, AttributeID, NodeGraph, Attribute, AttributeContents, DataKind};
use crate::state::{UserState, AppState, user_to_app_state};

use super::FerreData;

#[derive(PartialEq)]
enum GuiTab {
    Graph,
    Scene,
    Settings,
}

struct PinLayout {


}

pub struct NodeGui {
    input_pins: HashMap<AttributeID, egui::Rect>,
    dragged_pin: Option<AttributeID>,
    drag_delta: egui::Vec2,
    link_candidate: Option<AttributeID>,
    ferre_data: Option<FerreData>,
    current_tab: GuiTab,
    style: egui::style::Style,
    editor_offset: egui::Vec2,
    font_size: f32,
    top_area_h: f32,
    left_area_w: f32,
    scene_extent: wgpu::Extent3d,
    winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>,
}

impl NodeGui {
    pub fn new(winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>) -> Self {
        NodeGui {
            dragged_pin: None,
            drag_delta: egui::vec2(0.0, 0.0),
            link_candidate: None,
            input_pins: Default::default(),
            current_tab: GuiTab::Graph,
            font_size: 14.0,
            top_area_h: 0.0,
            style: Default::default(),
            editor_offset: egui::vec2(0.0, 0.0),
            left_area_w: 0.0,
            ferre_data: None,
            scene_extent: wgpu::Extent3d::default(),
            winit_proxy,
        }
    }

    fn add_textbox(&self, ui: &mut egui::Ui, label: &str, string: &mut String) -> egui::Response {
        let size = egui::vec2(self.font_size * 8.0, self.font_size);
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add_sized(size, egui::TextEdit::singleline(string))
        }).inner
    }

    fn add_input(&mut self, ui: &mut egui::Ui, id: AttributeID, label: &str, kind: &DataKind) {
        ui.horizontal(|ui| {
            let radius = ui.spacing().interact_size.y * 0.5;
            let desired_size =  radius * egui::vec2(1.0, 1.0);
            let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::drag());
            self.input_pins.insert(id, rect);

            // any pin can be a link candidate. We cannot use "response.hovered()"
            // because it does not register correctly if the pin is on another egui::Window.
            if ui.rect_contains_pointer(rect) {
                self.link_candidate = Some(id);
            }
            // if we are dragging
            if response.dragged_by(egui::PointerButton::Primary) {
                if response.drag_started() {
                    self.dragged_pin = Some(id);
                }
                self.drag_delta = response.drag_delta();
            }
            if response.drag_released() {
                if let Some(link_id) = self.link_candidate {
                    println!("linked to {}", link_id);
                    dbg!(link_id);
                } else {
                    println!("did not create a link!");
                }
                self.dragged_pin = None;
            }
            ui.painter().circle_filled(rect.center(), radius, egui::Color32::RED);
            ui.label(label);
        });
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
                            if ui.button("Render scene").clicked() {
                                let result = user_to_app_state(app_state, user_state);
                                if result.is_ok() {
                                    self.current_tab = GuiTab::Scene;
                                }
                            }
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

        // before looping over all nodes, reset a few variables
        self.link_candidate = None;

        for node_id in user_state.node_graph.get_node_ids() {
            // get all the useful information for our node
            struct Helper {
                pos: egui::Pos2,
                window_header: egui::WidgetText,
                attributes: Vec<AttributeID>,
            }
            let Helper {
                window_header,
                pos,
                attributes
            } = {
                let Node {
                    title,
                    position,
                    error,
                    contents
                } = user_state.node_graph.get_node_mut(node_id).unwrap();
                let window_header: egui::WidgetText = if let Some(_err) = error {
                    title.clone() + " ⚠"
                } else {
                    title.clone()
                }.into();
                let attributes = contents.get_attribute_list();
                Helper {
                    pos: egui::Pos2::from(*position),
                    window_header,
                    attributes
                }
            };

            let maybe_response = egui::Window::new(window_header)
                .id(egui::Id::new(node_id))
                .current_pos(pos + self.editor_offset)
                .auto_sized()
                .drag_bounds(egui::Rect::EVERYTHING)
                .show(ctx, |ui| {
                    self.add_node_contents(ui, &mut user_state.node_graph, &attributes);
                });
            if let Some(response) = maybe_response {
                let up_left = response.response.rect.min - self.editor_offset;
                let Node { position: pos, .. } = user_state.node_graph.get_node_mut(node_id).unwrap();
                (pos[0], pos[1]) = (up_left.x, up_left.y);

            }

        }
        // After rendering all the nodes, decide if we need to display a floating Bezier curve
        if let Some(start_id) = self.dragged_pin {
            match self.link_candidate {
                // draw between the two!
                Some(end_id) => {
                let painter = egui::Painter::new(ctx.clone(), egui::LayerId { order: egui::Order::PanelResizeLine, id: egui::Id::new("42") }, egui::Rect::EVERYTHING);
                painter.line_segment([self.input_pins.get(&start_id).unwrap().center(),
                                     self.input_pins.get(&end_id).unwrap().center()], egui::Stroke::new(1.0f32, egui::Color32::RED));
                },
                // draw between the first and the last known mouse position!
                None => {
                let pos =
                if let Some(pos) = ctx.input().pointer.hover_pos() {
                    pos
                } else {
                    egui::pos2(0.0, 0.0)
                };
                let painter = egui::Painter::new(ctx.clone(), egui::LayerId { order: egui::Order::Tooltip, id: egui::Id::new("42") }, egui::Rect::EVERYTHING);
                painter.line_segment([self.input_pins.get(&start_id).unwrap().center(),
                                     pos], egui::Stroke::new(1.0f32, egui::Color32::RED));

                }
            }
        }
        //egui::SidePanel::left("globals edit").show(ctx, |ui| {
        //    if ui.button("render scene from graph").clicked() {
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

    fn add_node_contents(&mut self, ui: &mut egui::Ui, graph: &mut NodeGraph, attributes: &[AttributeID]) {
        ui.vertical(|ui| {
            for id in attributes {
                let Attribute { node_id, contents } = graph.get_attribute_mut(*id).unwrap();
                match contents {
                    AttributeContents::Text { label, string } => {
                        self.add_textbox(ui, label.as_str(), string);
                    },
                    AttributeContents::InputPin { label, kind } => {
                        self.add_input(ui, *id, label, kind);
                    },
                    AttributeContents::OutputPin { label, kind } => {
                        ui.shrink_width_to_current();
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            ui.label(label.as_str());
                        });
                    },
                    _ => {ui.label(format!("attribute {} not yet supported", id));}
                }
            }
        });
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
