use std::collections::HashMap;

use egui::TextureId;
use pest::unicode::UPPERCASE_LETTER;

use crate::CustomEvent;
use crate::node_graph::{Node, NodeContents, AttributeID, NodeGraph, Attribute, AttributeContents, DataKind};
use crate::state::{UserState, AppState, user_to_app_state, user_state};

use super::FerreData;

#[derive(PartialEq)]
enum GuiTab {
    Graph,
    Scene,
    Settings,
}

struct GraphStyle {
    font_size: f32,
}

impl Default for GraphStyle {
    fn default() -> Self {
        GraphStyle {
            font_size: 14.0
        }
    }
}

#[derive(Default)]
struct GraphStatus {
    prev_link_candidate: Option<AttributeID>,
    new_link: Option<(AttributeID, AttributeID)>,
    link_candidate: Option<AttributeID>,
    pin_positions: HashMap<AttributeID, egui::Rect>,
    dragged_pin: Option<AttributeID>,
    drag_delta: egui::Vec2,
    editor_offset: egui::Vec2,
}

pub struct NodeGui {
    dragged_pin: Option<AttributeID>,
    drag_delta: egui::Vec2,
    ferre_data: Option<FerreData>,
    current_tab: GuiTab,
    style: egui::style::Style,
    graph_status: GraphStatus,
    graph_style: GraphStyle,
    top_area_h: f32,
    left_area_w: f32,
    scene_extent: wgpu::Extent3d,
    winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>,
}

impl NodeGui {
    pub fn new(winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>) -> Self {
        NodeGui {
            dragged_pin: None,
            graph_status: Default::default(),
            drag_delta: egui::vec2(0.0, 0.0),
            current_tab: GuiTab::Graph,
            graph_style: Default::default(),
            top_area_h: 0.0,
            style: Default::default(),
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

        show_node_editor(ctx, &mut self.graph_status, &self.graph_style, &mut user_state.node_graph);
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

fn add_node_contents(ui: &mut egui::Ui, status: &mut GraphStatus, style: &GraphStyle, graph: &mut NodeGraph, attributes: &[AttributeID]) {
    ui.vertical(|ui| {
        for id in attributes {
            let Attribute { node_id, contents } = graph.get_attribute_mut(*id).unwrap();
            match contents {
                AttributeContents::Text { label, string } => {
                    add_textbox(ui, status, style, label.as_str(), string);
                },
                AttributeContents::InputPin { label, kind } => {
                    add_input(ui, status, style, *id, label, *kind);
                },
                AttributeContents::OutputPin { label, kind } => {
                    add_output(ui, status, style, *id, label, *kind);
                },
                _ => {
                    ui.label(format!("attribute {} not yet supported", id));
                }
            };
        }
    });
}

fn add_textbox(ui: &mut egui::Ui, status: &mut GraphStatus, style: &GraphStyle, label: &str, string: &mut String) {
    let size = egui::vec2(style.font_size * 8.0, style.font_size);
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add_sized(size, egui::TextEdit::singleline(string))
    });
}

// Adding an output or an input pin are basically the same thing, only thing that changes is the
// "early shrink width to current" for output and the right-to-left or left-to-right layout.
// How the link is created also changes, but that needs to be done later on anyway.
fn add_input(ui: &mut egui::Ui, status: &mut GraphStatus, style: &GraphStyle, id: AttributeID, label: &str, kind: DataKind) {
    let layout = egui::Layout::left_to_right(egui::Align::TOP);
    add_pin(ui, status, style, layout, id, label, kind);
}

fn add_output(ui: &mut egui::Ui, status: &mut GraphStatus, style: &GraphStyle, id: AttributeID, label: &str, kind: DataKind) {
    ui.shrink_width_to_current();
    let layout = egui::Layout::right_to_left(egui::Align::TOP);
    add_pin(ui, status, style, layout, id, label, kind);
}

fn show_node_editor(ctx: &egui::Context, status: &mut GraphStatus, style: &GraphStyle, user_graph: &mut NodeGraph) {
    // before looping over all nodes, reset a few variables
    status.link_candidate = None;

    for node_id in user_graph.get_node_ids() {
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
            } = user_graph.get_node_mut(node_id).unwrap();
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

        // Show the window... Maybe! This is because the window might actually be closed, even if
        // in our case each window represents a Node, and shall NOT be closed.
        let maybe_response = egui::Window::new(window_header)
            .id(egui::Id::new(node_id))
            .current_pos(pos + status.editor_offset)
            .auto_sized()
            .drag_bounds(egui::Rect::EVERYTHING)
            .show(ctx, |ui| {
                add_node_contents(ui, status, style, user_graph, &attributes)
            });

        if let Some(result) = maybe_response {
            // Window was open: store the new position.
            let up_left = result.response.rect.min - status.editor_offset;
            let Node { position: pos, .. } = user_graph.get_node_mut(node_id).unwrap();
            (pos[0], pos[1]) = (up_left.x, up_left.y);
            // now we need to check if the window was minimized or not!
            match result.inner {
                // We have an inner return, therefore was not minimized
                Some(inner_link_candidate) => {
                    // we might want to do something else here
                },
                // No inner return: window was minimized!
                None => {
                    // We need to manually write to the rect positions!
                    // We want the same Y position for all, but a different X depending on it being
                    // an input or an output.
                    for id in attributes {
                        if user_graph.is_attribute_input(id) {
                            let resp_rect = result.response.rect;
                            let rect = egui::Rect {
                                min: resp_rect.min,
                                max: [resp_rect.left(), resp_rect.bottom()].into(),
                            };
                            status.pin_positions.insert(id, rect);
                        }
                        if user_graph.is_attribute_output(id) {
                            let resp_rect = result.response.rect;
                            let rect = egui::Rect {
                                min: [resp_rect.right(), resp_rect.top()].into(),
                                max: resp_rect.max,
                            };
                            status.pin_positions.insert(id, rect);
                        }
                    }
                }
            };
        }

    }
    // After rendering all the nodes, decide if we need to display a floating Bezier curve
    status.prev_link_candidate = status.link_candidate;
    let top_painter = egui::Painter::new(ctx.clone(), egui::LayerId { order: egui::Order::Tooltip, id: egui::Id::new("painter") }, egui::Rect::EVERYTHING);
    if let Some(start_id) = status.dragged_pin {
        match status.prev_link_candidate {
            // draw between the two!
            Some(end_id) => {
            top_painter.line_segment([status.pin_positions.get(&start_id).unwrap().center(),
                                 status.pin_positions.get(&end_id).unwrap().center()], egui::Stroke::new(1.0f32, egui::Color32::RED));
            },
            // draw between the first and the last known mouse position!
            None => {
            let pos =
            if let Some(pos) = ctx.input().pointer.hover_pos() {
                pos
            } else {
                egui::pos2(0.0, 0.0)
            };
            top_painter.line_segment([status.pin_positions.get(&start_id).unwrap().center(),
                                 pos], egui::Stroke::new(1.0f32, egui::Color32::RED));
            }
        }
    }

    match status.new_link {
        Some(pair) if user_graph.is_attribute_input(pair.0) => user_graph.insert_link(pair.0, pair.1),
        Some(pair) /*         the pair is reversed       */ => user_graph.insert_link(pair.1, pair.0),
        _ => {}
    }

    let mid_painter = egui::Painter::new(ctx.clone(), egui::LayerId { order: egui::Order::PanelResizeLine, id: egui::Id::new("painter") }, egui::Rect::EVERYTHING);
    // After the floating bezier, we need to show all the existing connections!
    for link in user_graph.get_links() {
        let in_pos = match status.pin_positions.get(link.0) {
            Some(rect) => rect.center(),
            _ => unreachable!(),
        };
        let out_pos = match status.pin_positions.get(link.1) {
            Some(rect) => rect.center(),
            _ => unreachable!(),
        };
        let pos_diff = in_pos.distance(out_pos);
        let delta = style.font_size * (0.375 + 0.375 * pos_diff.sqrt());
        let points = [out_pos, out_pos + (delta, 0.0).into(), in_pos + (-delta, 0.0).into(), in_pos];
        let shape = egui::epaint::CubicBezierShape::from_points_stroke(points, false, egui::Color32::default(), egui::Stroke::new(2.0f32, egui::Color32::RED));
        mid_painter.add(shape);
    }
    //egui::SidePanel::left("globals edit").show(ctx, |ui| {
    //    if ui.button("render scene from graph").clicked() {
    //    }
    //});
    egui::CentralPanel::default().show(ctx, |ui| {
        let (id, rect) = ui.allocate_space(ui.available_size());
        let response = ui.interact(rect, id, egui::Sense::click_and_drag());
        if response.dragged_by(egui::PointerButton::Middle) {
            status.editor_offset += response.drag_delta();
        }
    }); // central panel

}

fn add_pin(ui: &mut egui::Ui, status: &mut GraphStatus, style: &GraphStyle, layout: egui::Layout, id: AttributeID, label: &str, kind: DataKind) {
    ui.with_layout(layout, |ui| {
        let size = egui::Vec2::splat(style.font_size);
        let (response, painter) = ui.allocate_painter(size, egui::Sense::drag());
        let rect = response.rect;
        let c = rect.center();
        let r = rect.width() / 3.0;
        let color = egui::Color32::RED;
        painter.circle_filled(c, r, color);
        status.pin_positions.insert(id, rect);

        // any pin can be a link candidate. However we cannot use "response.hovered()"
        // because it does not register correctly if the pin is on another egui::Window.
        if ui.rect_contains_pointer(rect) {
            painter.circle_stroke(c, r, egui::Stroke::new(1.0, egui::Color32::GOLD));
            status.link_candidate = Some(id);
        }
        // if we are dragging
        if response.dragged_by(egui::PointerButton::Primary) {
            if response.drag_started() {
                status.dragged_pin = Some(id);
            }
            status.drag_delta = response.drag_delta();
        }
        // BEWARE: we need to use the link candidate from the PREVIOUS frame because while
        // looping widgets in the current frame, and we might not have gone through
        // the widget that is being hovered just yet!
        if response.drag_released() {
            if let Some(link_id) = status.prev_link_candidate {
                println!("linked to {}", link_id);
                status.new_link = Some((id, link_id));
                dbg!(link_id);
            } else {
                println!("did not create a link!");
            }
            status.dragged_pin = None;
        }
        ui.shrink_height_to_current();
        ui.horizontal_centered(|ui| ui.label(label));
    });
}

