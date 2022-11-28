use std::collections::HashMap;

use egui::TextureId;
use egui::collapsing_header::CollapsingState;
use pest::unicode::UPPERCASE_LETTER;

use crate::CustomEvent;
use crate::node_graph::{Node, NodeContents, AttributeID, NodeGraph, Attribute, AttributeContents, DataKind, SliderMode, AVAILABLE_SIZES, Axis};
use crate::node_graph::EguiId;
use crate::state::{UserState, AppState, user_to_app_state, user_state};

use super::{FerreData, Availables};

#[derive(PartialEq)]
enum GuiTab {
    Graph,
    Scene,
    Settings,
}

// BEWARE: DEFAULT_ZOOM controls how much "zoomed in" are the values that we read from
// the GraphNode data structure! It does not effect GUI directly.
const DEFAULT_ZOOM: f32 = ZOOM_SIZES[2];
const ZOOM_SIZES: [f32; 9] = [8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0];

struct GraphStatus {
    prev_link_candidate: Option<AttributeID>,
    new_link: Option<(AttributeID, AttributeID)>,
    link_candidate: Option<AttributeID>,
    pin_positions: HashMap<AttributeID, egui::Rect>,
    dragged_pin: Option<AttributeID>,
    drag_delta: egui::Vec2,
    editor_offset: egui::Vec2,
    zoom_level: usize,
    zoom_scrolling: f32,
    style: std::sync::Arc<egui::Style>,
}

// This is the default ratio between default font size and interaction size
const FONT_RATIO: f32 = 14.0/18.0;
// This is the default ratio between widget interaction width height
const WIDGET_RATIO: f32 = 40.0/18.0;
impl GraphStatus {
    fn new(zoom_level: usize) -> Self {
        let mut ret = Self {
            zoom_level,
            zoom_scrolling: 0.0,
            prev_link_candidate: None,
            new_link: None,
            link_candidate: None,
            pin_positions: HashMap::new(),
            dragged_pin: None,
            drag_delta: egui::vec2(0.0, 0.0),
            editor_offset: egui::vec2(0.0, 0.0),
            style: std::sync::Arc::new(egui::Style::default()),
        };
        ret.change_style_from_height(ZOOM_SIZES[ret.zoom_level]);
        ret
    }

    fn add_scrolling(&mut self, y: f32, cursor_pos: egui::Pos2) {
        self.zoom_scrolling += y;
        if self.zoom_scrolling > 49.0 {
            self.zoom_scrolling = 0.0;
            self.increment_zoom(cursor_pos);
        }
        if self.zoom_scrolling < -49.0 {
            self.zoom_scrolling = 0.0;
            self.decrement_zoom(cursor_pos);
        }
    }

    fn increment_zoom(&mut self, cursor_pos: egui::Pos2) {
        if self.zoom_level < ZOOM_SIZES.len() - 1 {
            // we want to change the editor offset to make sure that whatever was under the cursor
            // stays under the cursor. Compute the graph coordinates under the cursor
            let point_coords = self.rel_to_abs(cursor_pos);
            // z_ratio is defined as old_zoom / new_zoom
            let z_ratio = ZOOM_SIZES[self.zoom_level] / ZOOM_SIZES[self.zoom_level + 1];
            self.editor_offset = self.editor_offset * z_ratio + point_coords.to_vec2() * (z_ratio - 1.0);
            self.zoom_level += 1;
            self.change_style_from_height(ZOOM_SIZES[self.zoom_level]);
        }
    }

    fn decrement_zoom(&mut self, cursor_pos: egui::Pos2) {
        if self.zoom_level > 0 {
            let point_coords = self.rel_to_abs(cursor_pos);
            let z_ratio = ZOOM_SIZES[self.zoom_level] / ZOOM_SIZES[self.zoom_level - 1];
            self.editor_offset = self.editor_offset * z_ratio + point_coords.to_vec2() * (z_ratio - 1.0);
            self.zoom_level -= 1;
            self.change_style_from_height(ZOOM_SIZES[self.zoom_level]);
        }
    }

    fn change_style_from_height(&mut self, interact_height: f32) {
        // NOTE: make_mut will clone the style if it is currently in use
        // (for example, by the context!)
        // If this is the case, then this new_style will be used from the
            // NEXT frame onwards, which is what we want
        let new_style = std::sync::Arc::make_mut(&mut self.style);
        let monospace_font_id = egui::FontId {
            size: FONT_RATIO * interact_height,
            family: egui::FontFamily::Monospace,
        };
        new_style.text_styles.insert(egui::TextStyle::Button, monospace_font_id.clone());
        new_style.text_styles.insert(egui::TextStyle::Body, monospace_font_id.clone());
        new_style.override_font_id = Some(monospace_font_id);
        new_style.spacing.interact_size = egui::vec2(WIDGET_RATIO * interact_height, interact_height);
        new_style.spacing.slider_width = interact_height * 5.0;
        new_style.spacing.combo_height = interact_height;
        new_style.spacing.icon_width = interact_height * 1.0;
        new_style.spacing.icon_width_inner = interact_height * 0.75;
        new_style.spacing.indent = interact_height * 1.25;
        new_style.spacing.window_margin = egui::style::Margin {
            left: interact_height * 0.5,
            right: interact_height * 0.5,
            top: interact_height * 0.5,
            bottom: interact_height * 0.5,
        };
        new_style.spacing.item_spacing = egui::vec2(interact_height * 0.5, interact_height * 0.25);
        new_style.animation_time = 0.0;
        new_style.visuals.collapsing_header_frame = true;
    }

    fn add_node_contents(&mut self, ui: &mut egui::Ui, max_width: f32, availables: &Availables, graph: &mut NodeGraph, attributes: &[AttributeID]) {
        ui.vertical(|ui| {
            ui.set_max_width(max_width);
            ui.add_space(self.def_h() * 0.25);
            for id in attributes {
                let Attribute { contents, node_id: _ } = graph.get_attribute_mut(*id).unwrap();
                match contents {
                    AttributeContents::Text { label, string } => {
                        self.add_textbox(ui, label.as_str(), string);
                    }
                    AttributeContents::InputPin { label, kind } => {
                        self.add_input(ui, *id, label, *kind);
                    }
                    AttributeContents::OutputPin { label, kind } => {
                        self.add_output(ui, *id, label, *kind);
                    }
                    AttributeContents::IntSlider { label, value, mode } => {
                        self.add_slider(ui, label, value, mode);
                    }
                    AttributeContents::Color { label, color } => {
                        self.add_color_picker(ui, label, color);
                    }
                    AttributeContents::AxisSelect { axis } => {
                        self.add_axis_select(ui, *id, axis);
                    }
                    AttributeContents::MatrixRow { col_1, col_2, col_3, col_4 } => {
                        self.add_matrix_row(ui, [col_1, col_2, col_3, col_4]);
                    }
                    AttributeContents::Material { selected } => {
                        let uv_rect = egui::Rect {
                            min: (0.0, 0.0).into(),
                            max: (1.0, 1.0).into(),
                        };
                        self.add_texture_select(ui, *id, &availables.material_ids, selected, uv_rect);
                    }
                    AttributeContents::Mask { selected } => {
                        let uv_rect = egui::Rect {
                            min: (0.0, 0.0).into(),
                            max: (0.375, 0.375).into(),
                        };
                        self.add_texture_select(ui, *id, &availables.mask_ids, selected, uv_rect);
                    }
                    AttributeContents::PrimitiveKind { selected } => {
                        self.add_primitive_select(ui, *id, &availables.model_names, selected);
                    }
                    AttributeContents::Unknown { label }=> { ui.label(format!("unknown: {label}")); },
                };
            }
        });
    }

    fn def_h(&self) -> f32 {
        self.style.spacing.interact_size.y
    }

    fn add_primitive_select(&self, ui: &mut egui::Ui, attribute_id: AttributeID, model_names: &[&str], selected: &mut usize) {
        ui.horizontal(|ui| {
            ui.label("shape:");
            egui::ComboBox::from_id_source(attribute_id.new_egui_id())
                //.width(self.def_h() * 5.0)
                .selected_text(model_names[*selected])
                .show_ui(ui, |ui| {
                    for (idx, name) in model_names.iter().enumerate() {
                        ui.selectable_value(selected, idx, name.to_owned());
                    }
                });
        });
    }

    fn add_texture_select(&self, ui: &mut egui::Ui, attribute_id: AttributeID, material_ids: &[egui::TextureId], selected: &mut usize, uv_rect: egui::Rect) {
        let def_h = self.def_h();
        ui.horizontal(|ui| {
            ui.set_min_height(1.25 * def_h);
            ui.label("material");
            let popup_id = attribute_id.new_egui_id();
            let size = egui::Vec2::splat(1.25 * def_h);
            let img_button = egui::ImageButton::new(material_ids[*selected], size)
                .sense(egui::Sense::click_and_drag()) // Prevent node being moved by drag on the img
                .uv(uv_rect)
                .frame(false);
            let response = ui.add(img_button);
            if response.clicked() {
                ui.memory().open_popup(popup_id);
            }
            egui::popup::popup_below_widget(ui, popup_id, &response, |ui| {
                egui::Grid::new(popup_id.with(popup_id))
                    .min_col_width(0.0)
                    .spacing(egui::Vec2::splat(0.25 * def_h))
                    .show(ui, |ui| {
                    for (idx, texture) in material_ids.iter().enumerate() {
                        let button = egui::ImageButton::new(*texture, 2.0*size)
                            .uv(uv_rect)
                            .frame(false);
                        if ui.add(button).clicked() {
                            *selected = idx;
                        }
                        if idx % 4 == 3 {
                            ui.end_row();
                        }
                    }
                });
            });
        });
    }

    fn add_matrix_row(&mut self, ui: &mut egui::Ui, cols: [&mut String; 4]) {
        ui.horizontal(|ui| {
            // BEWARE: since we are inside a ui that was given a "max_width()" setting, in order to properly
            // show all the text edits of the same length we need to force their size with add_sized()
            let mut widget_size = ui.spacing().interact_size;
            widget_size.x = widget_size.y * 5.0;
            let col_1_edit = egui::TextEdit::singleline(cols[0]);
            ui.add_sized(widget_size, col_1_edit);

            let col_2_edit = egui::TextEdit::singleline(cols[1]);
            ui.add_sized(widget_size, col_2_edit);

            let col_3_edit = egui::TextEdit::singleline(cols[2]);
            ui.add_sized(widget_size, col_3_edit);

            let col_4_edit = egui::TextEdit::singleline(cols[3]);
            ui.add_sized(widget_size, col_4_edit);
        });
    }

    fn add_axis_select(&mut self, ui: &mut egui::Ui, id: AttributeID, axis: &mut Axis) {
        ui.horizontal(|ui| {
            ui.label("axis");
            egui::ComboBox::from_id_source(id)
                .selected_text(format!("{:?}", axis))
                .show_ui(ui, |ui| {
                    ui.selectable_value(axis, Axis::X, "X");
                    ui.selectable_value(axis, Axis::Y, "Y");
                    ui.selectable_value(axis, Axis::Z, "Z");
                }
            );
        });
    }

    fn add_color_picker(&mut self, ui: &mut egui::Ui, label: &str, color: &mut [f32; 3]) {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.color_edit_button_rgb(color);
        });
    }

    fn add_textbox(&mut self, ui: &mut egui::Ui, label: &str, string: &mut String) {
        let mut widget_size = ui.spacing().interact_size;
        widget_size.x = widget_size.y * 7.0;
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add_sized(widget_size, egui::TextEdit::singleline(string))
        });
    }

    fn add_slider(&mut self, ui: &mut egui::Ui, label: &str, value: &mut i32, mode: &mut SliderMode) {
        ui.horizontal(|ui| {
            ui.label(label);
            let slider = match mode {
                SliderMode::IntRange(min, max) => {
                    egui::Slider::new(value, *min ..= *max)
                        .show_value(false)
                },
                SliderMode::SizeLabels => {
                    egui::Slider::new(value, 0 ..= AVAILABLE_SIZES.len() as i32 - 1)
                        .show_value(false)
                        // We could use this custom formatter, however since egui uses an extra box to
                        // show the value, this would take way too much screen space
                        //.custom_formatter(|n, _| {
                        //    let idx = n as usize;
                        //    if let Some(thickness) = AVAILABLE_SIZES.get(idx) {
                        //        format!("{thickness}")
                        //    } else {
                        //        "0".to_string()
                        //    }
                        //})
                }
            };
            ui.add(slider);
        });
    }

    // Turn the absolute position to the relative one (i.e: the display one!)
    fn abs_to_rel(&self, pos: egui::Pos2) -> egui::Pos2 {
        let z_ratio = ZOOM_SIZES[self.zoom_level] / DEFAULT_ZOOM;
        let v = z_ratio * (pos.to_vec2() + self.editor_offset);
        v.to_pos2()
    }

    // Turn a relative position (the displayed one) to the absolute one (i.e: the one in graph_node data)
    fn rel_to_abs(&self, pos: egui::Pos2) -> egui::Pos2 {
        let z_ratio = ZOOM_SIZES[self.zoom_level] / DEFAULT_ZOOM;
        let v = pos.to_vec2() / z_ratio - self.editor_offset;
        v.to_pos2()
    }

    // Adding an output or an input pin are basically the same thing, only thing that changes is the
    // "early shrink width to current" for output and the right-to-left or left-to-right layout.
    // How the link is created also changes, but that needs to be done later on anyway.
    fn add_input(&mut self, ui: &mut egui::Ui, id: AttributeID, label: &str, kind: DataKind) {
        let layout = egui::Layout::left_to_right(egui::Align::TOP);
        self.add_pin(ui, layout, id, label, kind);
    }

    fn add_output(&mut self, ui: &mut egui::Ui, id: AttributeID, label: &str, kind: DataKind) {
        let layout = egui::Layout::right_to_left(egui::Align::TOP);
        self.add_pin(ui, layout, id, label, kind);
    }

    pub fn show_node_editor(&mut self, ctx: &egui::Context, avail_rect: egui::Rect, availables: &Availables, user_graph: &mut NodeGraph) {
        // First: override the style
        let prev_style = ctx.style();
        ctx.set_style(self.style.clone());
        // before looping over all nodes, reset a few variables
        self.link_candidate = None;

        // This is how each node will be rendered:
        // - we want a Window so that we have an area that can be easily moved around.
        // - we put a Collapsing state inside the window, because we want a more complete control of
        //   the header and the contents of the node.
        // - All the node attributes get rendered one after the other, in the same order as they are
        //   specified in the node_graph
        // Some specific gymnastics is needed to make the borrow checked happy, like the destructuring
        // and copying of the node contents into a helper structure.
        for node_id in user_graph.get_node_ids() {
            // get all the useful information for our node
            struct Helper {
                pos: egui::Pos2,
                window_title: egui::RichText,
                attributes: Vec<AttributeID>,
            }
            let Helper {
                window_title,
                pos,
                attributes
            } = {
                let Node {
                    title,
                    position,
                    error,
                    contents
                } = user_graph.get_node_mut(node_id).unwrap();
                let window_title: egui::RichText = if let Some(_err) = error {
                    title.clone() + " ⚠"
                } else {
                    title.clone()
                }.into();
                let attributes = contents.get_attribute_list();
                Helper {
                    pos: egui::Pos2::from(*position),
                    window_title,
                    attributes
                }
            };

            // When you show the window, it might actually return None if the window itself was closed.
            // In our case each window represents a Node, and should NOT be closed.
            // We might want to change this in the future, if we create a way to group windows.
            let position = self.abs_to_rel(pos);
            let window_return = egui::Window::new("") // NO window title, since we set a unique ID
                .id(node_id.new_egui_id())
                .current_pos(position)
                .auto_sized()
                .title_bar(false)
                .drag_bounds(egui::Rect::EVERYTHING)
                .show(ctx, |ui| {
                    let header_builder = CollapsingState::load_with_default_open(ctx, ui.make_persistent_id(node_id), true);
                    let mut max_width = 0.0;
                    let first_response = header_builder.show_header(ui, |ui| {
                        ui.strong(window_title);
                        max_width = ui.min_size().x
                    });
                    let body_return = first_response.body_unindented(|ui| {
                        // TODO: is this little change of visuals worth it?
                        //egui::Frame::none()
                        //    .fill(self.style.visuals.faint_bg_color)
                        //    .inner_margin(egui::style::Margin::from(2.0))
                        //    .outer_margin(egui::style::Margin::from(-2.0))
                        //    .rounding(self.style.visuals.window_rounding)
                        //    .show(ui, |ui| {
                                self.add_node_contents(ui, max_width, availables, user_graph, &attributes)
                        //});
                    });
                    // We can now check if the header was collapsed. If it was, then no data was
                    // writtend for the pin locations, and we need to manually set them
                    let maybe_inner_response = body_return.2;

                    if maybe_inner_response.is_none() {
                        // We want the same Y position for all, but a different X for inputs and outputs.
                        for id in attributes {
                            if user_graph.is_attribute_input(id) {
                                let resp_rect = body_return.0.rect; // Use the collapse button response
                                let rect = egui::Rect {
                                    min: resp_rect.min,
                                    max: [resp_rect.left(), resp_rect.bottom()].into(),
                                };
                                self.pin_positions.insert(id, rect);
                            }
                            if user_graph.is_attribute_output(id) {
                                let resp_rect = body_return.1.response.rect; // Use the header response
                                let rect = egui::Rect {
                                    min: [resp_rect.right(), resp_rect.top()].into(),
                                    max: resp_rect.max,
                                };
                                self.pin_positions.insert(id, rect);
                            }
                        }

                    }
                }).expect("no egui::Window should ever be closed inside the node editor"); // Assume egui::Window is open

            // After we have the window response, we can assume that the window was not collapsed
            // (in fact, the user cannot collapse it because the window itself has no title bar)
            // We can store its position!
            let window_response = window_return.response;
            let up_left = self.rel_to_abs(window_response.rect.min);
            let Node { position: pos, .. } = user_graph.get_node_mut(node_id).unwrap();
            (pos[0], pos[1]) = (up_left.x, up_left.y);
        }
        // After rendering all the nodes, decide if we need to display a floating Bezier curve
        self.prev_link_candidate = self.link_candidate;
        let top_painter = egui::Painter::new(ctx.clone(), egui::LayerId { order: egui::Order::Tooltip, id: egui::Id::new("painter") }, egui::Rect::EVERYTHING);
        if let Some(start_id) = self.dragged_pin {
            match self.prev_link_candidate {
                // draw between the two!
                Some(end_id) => {
                top_painter.line_segment([self.pin_positions.get(&start_id).unwrap().center(),
                                     self.pin_positions.get(&end_id).unwrap().center()], egui::Stroke::new(1.0f32, egui::Color32::RED));
                },
                // draw between the first and the last known mouse position!
                None => {
                let pos =
                if let Some(pos) = ctx.input().pointer.hover_pos() {
                    pos
                } else {
                    egui::pos2(0.0, 0.0)
                };
                top_painter.line_segment([self.pin_positions.get(&start_id).unwrap().center(),
                                     pos], egui::Stroke::new(1.0f32, egui::Color32::RED));
                }
            }
        }

        match self.new_link {
            Some(pair) if user_graph.is_attribute_input(pair.0) => user_graph.insert_link(pair.0, pair.1),
            Some(pair) /*        the pair is reversed        */ => user_graph.insert_link(pair.1, pair.0),
            _ => {}
        }

        let mid_painter = egui::Painter::new(ctx.clone(), egui::LayerId { order: egui::Order::PanelResizeLine, id: egui::Id::new("painter") }, egui::Rect::EVERYTHING);
        // After the floating bezier, we need to show all the existing connections!
        for link in user_graph.get_links() {
            let in_pos = match self.pin_positions.get(link.0) {
                Some(rect) => rect.center(),
                _ => unreachable!(),
            };
            let out_pos = match self.pin_positions.get(link.1) {
                Some(rect) => rect.center(),
                _ => unreachable!(),
            };
            let real_distance = in_pos.distance(out_pos) * DEFAULT_ZOOM / ZOOM_SIZES[self.zoom_level];
            let delta = self.def_h() * (0.25 + 0.5 * real_distance.sqrt());
            let points = [out_pos, out_pos + (delta, 0.0).into(), in_pos + (-delta, 0.0).into(), in_pos];
            let shape = egui::epaint::CubicBezierShape::from_points_stroke(points, false, egui::Color32::default(), egui::Stroke::new(2.0f32, egui::Color32::RED));
            mid_painter.add(shape);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let (id, rect) = ui.allocate_space(ui.available_size());
            let response = ui.interact(rect, id, egui::Sense::click_and_drag());
            if response.dragged_by(egui::PointerButton::Middle) {
                self.editor_offset += response.drag_delta() * DEFAULT_ZOOM / ZOOM_SIZES[self.zoom_level];
            }

            let ctrl_down = ctx.input().modifiers.ctrl;
            if response.dragged_by(egui::PointerButton::Primary) && ctrl_down {
                self.editor_offset += response.drag_delta() * DEFAULT_ZOOM / ZOOM_SIZES[self.zoom_level];
            }

            let maybe_hover = ctx.input().pointer.hover_pos();
            if let Some(hover_pos) = maybe_hover {
                if avail_rect.contains(hover_pos) {
                    for event in ctx.input().events.iter() {
                        match event {
                            // We want to respond both to mouse wheel scrolling and pinch zoom
                            egui::Event::Scroll(v) => self.add_scrolling(v.y, hover_pos),
                            egui::Event::Zoom(z) => self.add_scrolling((*z - 1.0) * 20.0, hover_pos),
                            _ => {},
                        }
                    }
                }
            }
        }); // central panel

        // Finally: reset the style to the standard one
        ctx.set_style(prev_style);
    }

    fn add_pin(&mut self, ui: &mut egui::Ui, layout: egui::Layout, id: AttributeID, label: &str, kind: DataKind) {
        ui.with_layout(layout, |ui| {
            let size = egui::Vec2::splat(ui.spacing().interact_size.y);
            let (response, painter) = ui.allocate_painter(size, egui::Sense::drag());
            let rect = response.rect;
            let c = rect.center();
            let r = rect.width() / 3.0;
            let color = egui::Color32::RED;
            painter.circle_filled(c, r, color);
            self.pin_positions.insert(id, rect);

            // any pin can be a link candidate. However we cannot use "response.hovered()"
            // because it does not register correctly if the pin is on another egui::Window.
            if ui.rect_contains_pointer(rect) {
                painter.circle_stroke(c, r, egui::Stroke::new(1.0, egui::Color32::GOLD));
                self.link_candidate = Some(id);
            }
            // if we are dragging
            if response.dragged_by(egui::PointerButton::Primary) {
                if response.drag_started() {
                    self.dragged_pin = Some(id);
                }
                self.drag_delta = response.drag_delta();
            }
            // BEWARE: we need to use the link candidate from the PREVIOUS frame because while
            // looping widgets in the current frame, and we might not have gone through
            // the widget that is being hovered just yet!
            if response.drag_released() {
                if let Some(link_id) = self.prev_link_candidate {
                    println!("linked to {}", link_id);
                    self.new_link = Some((id, link_id));
                    dbg!(link_id);
                } else {
                    println!("did not create a link!");
                }
                self.dragged_pin = None;
            }
            ui.shrink_height_to_current();
            ui.horizontal_centered(|ui| ui.label(label));
        });
    }


}

pub struct NodeGui {
    availables: Availables,
    dragged_pin: Option<AttributeID>,
    drag_delta: egui::Vec2,
    ferre_data: Option<FerreData>,
    current_tab: GuiTab,
    style: egui::style::Style,
    graph_status: GraphStatus,
    top_area_h: f32,
    left_area_w: f32,
    scene_extent: wgpu::Extent3d,
    winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>,
}

impl NodeGui {
    pub fn new(winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>, availables: Availables) -> Self {
        NodeGui {
            dragged_pin: None,
            graph_status: GraphStatus::new(3),
            availables,
            drag_delta: egui::vec2(0.0, 0.0),
            current_tab: GuiTab::Graph,
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

    fn show_graph_tab(&mut self, ctx: &egui::Context, avail_rect: egui::Rect, app_state: &mut AppState, user_state: &mut UserState) {
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
                            if ui.button("increase zoom").clicked() {
                                self.graph_status.increment_zoom(egui::pos2(0.0, 0.0));
                            }
                            if ui.button("Decrease graph").clicked() {
                                self.graph_status.decrement_zoom(egui::pos2(0.0, 0.0));
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
        let avail_rect = egui::Rect {
            min: egui::pos2(used_x, avail_rect.top()),
            max: ctx.available_rect().max
            //max: egui::pos2(std::f32::INFINITY, std::f32::INFINITY),
        };

        self.graph_status.show_node_editor(ctx, avail_rect, &self.availables, &mut user_state.node_graph);
    }

    fn show_scene_tab(&mut self, ctx: &egui::Context, app_state: &mut AppState, texture_id: TextureId) {
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

    fn show_settings_tab(&mut self, ctx: &egui::Context, app_state: &mut AppState) {
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
            //max: egui::pos2(std::f32::INFINITY, std::f32::INFINITY),
        };
        match self.current_tab {
            GuiTab::Graph => self.show_graph_tab(ctx, avail_rect, app_state, user_state),
            GuiTab::Scene => self.show_scene_tab(ctx, app_state, texture_id),
            GuiTab::Settings => self.show_settings_tab(ctx, app_state),
        }
    }

    fn load_ferre_data(&mut self, ctx: &egui::Context, ferre_data: FerreData) {
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

