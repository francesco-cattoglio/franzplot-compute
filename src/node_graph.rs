use std::collections::HashMap;
use crate::cpp_gui::imnodes;
use crate::cpp_gui::PinShape;
use crate::rust_gui::Availables;
use serde::{Serialize, Deserialize};
use imgui::*;

pub type AttributeID = i32;
pub type NodeID = i32;

#[derive(Clone, PartialEq, Deserialize, Serialize, Debug,)]
pub enum DataKind {
    Interval,
    Geometry,
    Matrix,
    Vector,
}

pub const ZOOM_LEVELS: [f32; 6] = [1.0, 0.8, 0.64, 0.512, 0.41, 0.32];

fn create_style_shim(scale: f32) -> imnodes::StyleShim {
    imnodes::StyleShim {
        grid_spacing: 32.0 * scale,
        node_padding_horizontal: 8.0 * scale,
        node_padding_vertical: 8.0 * scale,
        pin_circle_radius: 5.0 * scale,
        pin_quad_side_length: 8.0 * scale,
        pin_triangle_side_length: 12.0 * scale,
        pin_line_thickness: 1.0 * scale,
        pin_hover_radius: 10.0 * scale,
        link_thickness: 4.0 * scale,
    }
}

impl DataKind {
    // we might even return a color as well!
    fn to_pin_shape(&self) -> i32 {
        let pin_shape = match self {
            DataKind::Interval => PinShape::QuadFilled,
            DataKind::Geometry => PinShape::CircleFilled,
            DataKind::Vector => PinShape::TriangleFilled,
            DataKind::Matrix => PinShape::Quad,
        };
        pin_shape as i32
    }
}

#[derive(Clone, Deserialize, Serialize, Debug,)]
pub struct Attribute {
    node_id: NodeID,
    contents: AttributeContents,
}

#[derive(Clone, Deserialize, Serialize, Debug,)]
pub enum SliderMode {
    IntRange(i32, i32),
    SizeLabels,
}

#[derive(Copy, Clone, Deserialize, Serialize, Debug,)]
pub enum Axis {
    X,
    Y,
    Z,
}

#[derive(Clone, Deserialize, Serialize, Debug,)]
pub enum AttributeContents {
    InputPin {
        label: String,
        kind: DataKind,
    },
    OutputPin {
        label: String,
        kind: DataKind,
    },
    Text {
        label: String,
        string: String,
    },
    IntSlider {
        label: String,
        value: i32,
        mode: SliderMode,
    },
    MatrixRow {
        col_1: String,
        col_2: String,
        col_3: String,
        col_4: String,
    },
    AxisSelect {
        axis: Axis,
    },
    Color {
        label: String,
        color: [f32; 3],
    },
    Mask {
        selected: usize,
    },
    Material {
        selected: usize,
    },
    PrimitiveKind {
        selected: usize,
    },
    Unknown {
        label: String,
    }
}

pub const AVAILABLE_SIZES: [f32; 9] = [0.04, 0.08, 0.12, 0.16, 0.20, 0.24, 0.32, 0.4, 0.8];

impl Attribute {
    // the render function shall return bool if anything has changed.
    pub fn render(&mut self, ui: &imgui::Ui<'_>, availables: &Availables, id: AttributeID) -> bool {
        // TODO: maybe we can push the style var at the begin of the editor rendering,
        // just like we push the imnodes style vars
        let font_size = ui.current_font_size();
        let style_token = ui.push_style_var(StyleVar::ItemSpacing([0.24 * font_size, 0.26 * font_size]));
        let [char_w, _char_h] = ui.calc_text_size("A");
        let value_changed = match &mut self.contents {
            AttributeContents::InputPin {
                label, kind,
            } => {
                imnodes::BeginInputAttribute(id, kind.to_pin_shape());
                ui.text(label);
                imnodes::EndInputAttribute();
                false
            },
            AttributeContents::OutputPin {
                label, kind,
            } => {
                imnodes::BeginOutputAttribute(id, kind.to_pin_shape());
                ui.text(label);
                imnodes::EndOutputAttribute();
                false
            },
            AttributeContents::Text {
                label, string,
            } => {
                let widget_width = 16.5 * char_w;

                imnodes::BeginStaticAttribute(id);
                ui.text(&label);
                ui.same_line();
                ui.set_next_item_width(widget_width);
                let value_changed = InputText::new(ui, "", &mut *string)
                    .no_undo_redo(true)
                    .build();
                imnodes::EndStaticAttribute();
                value_changed
            },
            AttributeContents::AxisSelect {
                axis
            } => {
                let widget_width = 8.5 * char_w;

                imnodes::BeginStaticAttribute(id);

                ui.text(" axis");
                ui.same_line();
                ui.set_next_item_width(widget_width);
                let choices = vec!("X", "Y", "Z");
                let mut selected = match axis {
                    Axis::X => 0,
                    Axis::Y => 1,
                    Axis::Z => 2,
                };
                ui.text("TODO: combo box");
                let value_changed = false;
                //let value_changed = ComboBox::new("##axis")
                //    .build_simple_string(ui, &mut selected, &choices);
                //*axis = match selected {
                //    0 => Axis::X,
                //    1 => Axis::Y,
                //    2 => Axis::Z,
                //    _ => panic!()
                //};
                imnodes::EndStaticAttribute();
                value_changed
            },
            AttributeContents::IntSlider {
                label, value, mode,
            } => {
                let widget_width = 12.0 * char_w;

                imnodes::BeginStaticAttribute(id);
                ui.text(&label);
                ui.same_line();
                ui.set_next_item_width(widget_width);
                let mut value_changed = match mode {
                    SliderMode::IntRange(min, max) => {
                        Slider::new("", *min, *max)
                            .flags(SliderFlags::NO_INPUT)
                            .build(ui, value)
                    },
                    SliderMode::SizeLabels => {
                        let max_id = AVAILABLE_SIZES.len() - 1;
                        let string_id = max_id.min(*value as usize);
                        let display_string: String = format!("{}", AVAILABLE_SIZES[string_id]).into();
                        Slider::new("", 0, max_id as i32)
                            .display_format(&display_string)
                            .flags(SliderFlags::NO_INPUT)
                            .build(ui, value)
                    }
                };
                imnodes::EndStaticAttribute();
                value_changed
            },
            AttributeContents::MatrixRow {
                col_1, col_2, col_3, col_4,
            } => {
                let mut value_changed = false;
                imnodes::BeginStaticAttribute(id);

                let widget_width = 8.5 * char_w;

                ui.set_next_item_width(widget_width);
                value_changed |= InputText::new(ui, "##1", &mut *col_1)
                    .no_undo_redo(true)
                    .build();
                ui.same_line();

                ui.set_next_item_width(widget_width);

                value_changed |= InputText::new(ui, "##2", &mut *col_2)
                    .no_undo_redo(true)
                    .build();

                ui.same_line();

                ui.set_next_item_width(widget_width);
                value_changed |= InputText::new(ui, "##3", &mut *col_3)
                    .no_undo_redo(true)
                    .build();

                ui.same_line();

                ui.set_next_item_width(widget_width);
                value_changed |= InputText::new(ui, "##4", &mut *col_4)
                    .no_undo_redo(true)
                    .build();

                imnodes::EndStaticAttribute();
                value_changed
            },
            AttributeContents::Color {
                label, color
            } => {
                let widget_width = 16.5 * char_w;

                imnodes::BeginStaticAttribute(id);
                ui.text(&label);
                ui.same_line();
                let color_picker = ColorEdit::new("", EditableColor::Float3(color))
                    .inputs(false)
                    .options(true);

                ui.set_next_item_width(widget_width);
                let value_changed = color_picker.build(ui);
                imnodes::EndStaticAttribute();
                value_changed
            },
            AttributeContents::Mask {
                selected
            } => {
                let widget_width = 2.35 * char_w;

                imnodes::BeginStaticAttribute(id);
                ui.text("Mask:");
                ui.same_line();
                let mut value_changed = false;
                // clamp the value of "selected" to the masks vector length
                if *selected >= availables.mask_ids.len() {
                    value_changed = true;
                    *selected = availables.mask_ids.len() - 1;
                }
                let button = ImageButton::new(availables.mask_ids[*selected], [widget_width, widget_width])
                    .uv1([0.5, 0.5]) // the pattern will be zoomed in by showing only a small part
                    .frame_padding(0);
                if button.build(ui) {
                    ui.open_popup("mask selection");
                }

                let mut new_selection: Option<usize> = None;
                let token = ui.push_style_var(StyleVar::WindowPadding([4.0, 4.0]));
                ui.popup("mask selection", || {
                    for (i, texture) in availables.mask_ids.iter().enumerate() {
                        if i%4 != 0 {
                            ui.same_line();
                        }
                        let button = ImageButton::new(*texture, [32.0, 32.0])
                            .frame_padding(0);
                        if button.build(ui) {
                            new_selection = Some(i);
                            dbg!(&new_selection);
                            ui.close_current_popup();
                        }
                    }
                });
                token.pop();
                if let Some(user_selection) = new_selection {
                    if *selected != user_selection {
                        *selected = user_selection;
                        value_changed = true;
                    }
                }
                imnodes::EndStaticAttribute();
                value_changed
            },
            AttributeContents::Material {
                selected
            } => {
                let widget_width = 2.35 * char_w;

                imnodes::BeginStaticAttribute(id);
                ui.text("Material:");
                ui.same_line();
                let mut value_changed = false;
                // clamp the value of "selected" to the materials length
                if *selected >= availables.material_ids.len() {
                    value_changed = true;
                    *selected = availables.material_ids.len() - 1;
                }
                let button = ImageButton::new(availables.material_ids[*selected], [widget_width, widget_width])
                    .frame_padding(0);
                if button.build(ui) {
                    ui.open_popup("material selection");
                }

                let mut new_selection: Option<usize> = None;
                let token = ui.push_style_var(StyleVar::WindowPadding([4.0, 4.0]));
                ui.popup("material selection", || {
                    for (i, texture) in availables.material_ids.iter().enumerate() {
                        if i%4 != 0 {
                            ui.same_line();
                        }
                        let button = ImageButton::new(*texture, [32.0, 32.0])
                            .frame_padding(0);
                        if button.build(ui) {
                            new_selection = Some(i);
                            dbg!(&new_selection);
                            ui.close_current_popup();
                        }
                    }
                });
                token.pop();
                if let Some(user_selection) = new_selection {
                    if *selected != user_selection {
                        *selected = user_selection;
                        value_changed = true;
                    }
                }
                imnodes::EndStaticAttribute();
                value_changed
            },
            AttributeContents::PrimitiveKind {
                selected
            } => {
                let widget_width = 12.0 * char_w;

                imnodes::BeginStaticAttribute(id);
                ui.text("Kind:");
                ui.same_line();
                ui.set_next_item_width(widget_width);
                let mut value_changed = false;
                let list: Vec<&ImString> = availables.model_names.iter().collect();
                ui.text("TODO: combo box");
                //if ComboBox::new("##primitive").build_simple_string(ui, selected, &list) {
                //    value_changed = true;
                //}
                imnodes::EndStaticAttribute();
                value_changed
            },
            AttributeContents::Unknown {
                ..
            } => {
                unimplemented!()
            }
        };
        style_token.pop();
        value_changed
    }

    pub fn render_list(ui: &imgui::Ui<'_>, availables: &Availables, attributes: &mut Vec<Option<Attribute>>, attribute_id_list: Vec<AttributeID>) -> bool {
        let mut value_changed = false;
        for id in attribute_id_list.into_iter() {
            if let Some(Some(attribute)) = attributes.get_mut(id as usize) {
                value_changed |= attribute.render(ui, availables, id);
            }
        }
        value_changed
    }
}

#[derive(Clone, Deserialize, Serialize, Debug,)]
pub enum NodeContents {
    Interval {
        variable: AttributeID,
        begin: AttributeID,
        end: AttributeID,
        quality: AttributeID,
        output: AttributeID,
    },
    Sample {
        geometry: AttributeID,
        parameter: AttributeID,
        value: AttributeID,
        output: AttributeID,
    },
    Vector {
        x: AttributeID,
        y: AttributeID,
        z: AttributeID,
        output: AttributeID,
    },
    Point {
        x: AttributeID,
        y: AttributeID,
        z: AttributeID,
        output: AttributeID,
    },
    Bezier {
        p0: AttributeID,
        p1: AttributeID,
        p2: AttributeID,
        p3: AttributeID,
        quality: AttributeID,
        output: AttributeID,
    },
    Curve {
        interval: AttributeID,
        fx: AttributeID,
        fy: AttributeID,
        fz: AttributeID,
        output: AttributeID,
    },
    Surface {
        interval_1: AttributeID,
        interval_2: AttributeID,
        fx: AttributeID,
        fy: AttributeID,
        fz: AttributeID,
        output: AttributeID,
    },
    Plane {
        center: AttributeID,
        normal: AttributeID,
        size: AttributeID,
        output: AttributeID,
    },
    Transform {
        geometry: AttributeID,
        matrix: AttributeID,
        output: AttributeID,
    },
    Matrix {
        interval: AttributeID,
        row_1: AttributeID,
        row_2: AttributeID,
        row_3: AttributeID,
        output: AttributeID,
    },
    RotationMatrix {
        axis: AttributeID,
        angle: AttributeID,
        output: AttributeID,
    },
    TranslationMatrix {
        vector: AttributeID,
        output: AttributeID,
    },
    Rendering {
        geometry: AttributeID,
        thickness: AttributeID,
        mask: AttributeID,
        material: AttributeID,
    },
    VectorRendering {
        application_point: AttributeID,
        vector: AttributeID,
        thickness: AttributeID,
        material: AttributeID,
    },
    Primitive {
        primitive: AttributeID,
        size: AttributeID,
        output: AttributeID,
    },
    Group
}

impl NodeContents {
    pub fn default_same_kind(&self) -> Self {
        match self {
            NodeContents::Interval {..} => Self::default_interval(),
            NodeContents::Sample {..} => Self::default_sample(),
            NodeContents::Vector {..} => Self::default_vector(),
            NodeContents::Point {..} => Self::default_point(),
            NodeContents::Bezier {..} => Self::default_bezier(),
            NodeContents::Curve {..} => Self::default_curve(),
            NodeContents::Surface {..} => Self::default_surface(),
            NodeContents::Plane {..} => Self::default_plane(),
            NodeContents::Matrix {..} => Self::default_matrix(),
            NodeContents::RotationMatrix {..} => Self::default_rotation_matrix(),
            NodeContents::TranslationMatrix {..} => Self::default_translation_matrix(),
            NodeContents::Transform {..} => Self::default_transform(),
            NodeContents::Rendering {..} => Self::default_rendering(),
            NodeContents::VectorRendering {..} => Self::default_vector_rendering(),
            NodeContents::Primitive {..} => Self::default_primitive(),
            NodeContents::Group => unimplemented!(),
        }
    }

    // NOTE: it is very important that we keep the order in which we return the attributes
    // with the order of attributes returned in the NodeContents::default_*() functions!
    pub fn get_attribute_list_mut(&mut self) -> Vec<&mut AttributeID> {
        match self {
            NodeContents::Interval {
                variable, begin, end, quality, output,
            } => {
                vec![variable, begin, end, quality, output]
            },
            NodeContents::Sample {
                geometry, parameter, value, output,
            } => {
                vec![geometry, parameter, value, output]
            },
            NodeContents::Vector {
                x, y, z, output
            } => {
                vec![x, y, z, output]
            },
            NodeContents::Point {
                x, y, z, output
            } => {
                vec![x, y, z, output]
            },
            NodeContents::Bezier {
                p0, p1, p2, p3, quality, output
            } => {
                vec![p0, p1, p2, p3, quality, output]
            },
            NodeContents::Curve {
                interval, fx, fy, fz, output
            } => {
                vec![interval, fx, fy, fz, output]
            },
            NodeContents::Surface {
                interval_1, interval_2, fx, fy, fz, output
            } => {
                vec![interval_1, interval_2, fx, fy, fz, output]
            },
            NodeContents::Plane {
                center, normal, size, output
            } => {
                vec![center, normal, size, output]
            },
            NodeContents::Transform {
                geometry, matrix, output
            } => {
                vec![geometry, matrix, output]
            },
            NodeContents::Matrix {
                interval, row_1, row_2, row_3, output,
            } => {
                vec![interval, row_1, row_2, row_3, output,]
            },
            NodeContents::RotationMatrix {
                axis, angle, output,
            } => {
                vec![axis, angle, output,]
            },
            NodeContents::TranslationMatrix {
                vector, output,
            } => {
                vec![vector, output,]
            },
            NodeContents::Rendering {
                geometry, thickness, mask, material,
            } => {
                vec![geometry, thickness, mask, material,]
            },
            NodeContents::VectorRendering {
                application_point, vector, thickness, material,
            } => {
                vec![application_point, vector, thickness, material,]
            },
            NodeContents::Primitive {
                primitive, size, output,
            } => {
                vec![primitive, size, output,]
            },
            NodeContents::Group => {
                unimplemented!()
            }
        }
    }

    // NOTE: it is very important that we keep the order in which we return the attributes
    // with the order of attributes returned in the NodeContents::default_*() functions!
    pub fn get_attribute_list(&self) -> Vec<AttributeID> {
        match *self {
            NodeContents::Interval {
                variable, begin, end, quality, output,
            } => {
                vec![variable, begin, end, quality, output]
            },
            NodeContents::Sample {
                geometry, parameter, value, output,
            } => {
                vec![geometry, parameter, value, output]
            },
            NodeContents::Vector {
                x, y, z, output
            } => {
                vec![x, y, z, output]
            },
            NodeContents::Point {
                x, y, z, output
            } => {
                vec![x, y, z, output]
            },
            NodeContents::Bezier {
                p0, p1, p2, p3, quality, output
            } => {
                vec![p0, p1, p2, p3, quality, output]
            },
            NodeContents::Curve {
                interval, fx, fy, fz, output
            } => {
                vec![interval, fx, fy, fz, output]
            },
            NodeContents::Surface {
                interval_1, interval_2, fx, fy, fz, output
            } => {
                vec![interval_1, interval_2, fx, fy, fz, output]
            },
            NodeContents::Plane {
                center, normal, size, output
            } => {
                vec![center, normal, size, output]
            },
            NodeContents::Transform {
                geometry, matrix, output
            } => {
                vec![geometry, matrix, output]
            },
            NodeContents::Matrix {
                interval, row_1, row_2, row_3, output,
            } => {
                vec![interval, row_1, row_2, row_3, output,]
            },
            NodeContents::RotationMatrix {
                axis, angle, output,
            } => {
                vec![axis, angle, output,]
            },
            NodeContents::TranslationMatrix {
                vector, output,
            } => {
                vec![vector, output,]
            },
            NodeContents::Rendering {
                geometry, thickness, mask, material,
            } => {
                vec![geometry, thickness, mask, material]
            },
            NodeContents::VectorRendering {
                application_point, vector, thickness, material,
            } => {
                vec![application_point, vector, thickness, material,]
            },
            NodeContents::Primitive {
                primitive, size, output,
            } => {
                vec![primitive, size, output,]
            },
            NodeContents::Group => {
                unimplemented!()
            }
        }
    }

    // NOTE: if you modify this function, also modify the order in which we return
    // attributes in the get_attribute_list_mut() and get_attribute_list() functions!
    pub fn default_interval() -> Self {
        NodeContents::Interval {
            variable: 0,
            begin: 1,
            end: 2,
            quality: 3,
            output: 4,
        }
    }

    // NOTE: if you modify this function, also modify the order in which we return
    // attributes in the get_attribute_list_mut() and get_attribute_list() functions!
    pub fn default_sample() -> Self {
        NodeContents::Sample {
            geometry: 0,
            parameter: 1,
            value: 2,
            output: 3,
        }
    }

    // NOTE: if you modify this function, also modify the order in which we return
    // attributes in the get_attribute_list_mut() and get_attribute_list() functions!
    pub fn default_vector() -> Self {
        NodeContents::Vector {
            x: 0,
            y: 1,
            z: 2,
            output: 3,
        }
    }

    // NOTE: if you modify this function, also modify the order in which we return
    // attributes in the get_attribute_list_mut() and get_attribute_list() functions!
    pub fn default_point() -> Self {
        NodeContents::Point {
            x: 0,
            y: 1,
            z: 2,
            output: 3,
        }
    }

    // NOTE: if you modify this function, also modify the order in which we return
    // attributes in the get_attribute_list_mut() and get_attribute_list() functions!
    pub fn default_bezier() -> Self {
        NodeContents::Bezier {
            p0: 0,
            p1: 1,
            p2: 2,
            p3: 3,
            quality: 4,
            output: 5,
        }
    }

    // NOTE: if you modify this function, also modify the order in which we return
    // attributes in the get_attribute_list_mut() and get_attribute_list() functions!
    pub fn default_curve() -> Self {
        NodeContents::Curve {
            interval: 0,
            fx: 1,
            fy: 2,
            fz: 3,
            output: 4,
        }
    }

    // NOTE: if you modify this function, also modify the order in which we return
    // attributes in the get_attribute_list_mut() and get_attribute_list() functions!
    pub fn default_surface() -> Self {
        NodeContents::Surface {
            interval_1: 0,
            interval_2: 1,
            fx: 2,
            fy: 3,
            fz: 4,
            output: 5,
        }
    }

    // NOTE: if you modify this function, also modify the order in which we return
    // attributes in the get_attribute_list_mut() and get_attribute_list() functions!
    pub fn default_plane() -> Self {
        NodeContents::Plane {
            center: 0,
            normal: 1,
            size: 2,
            output: 3,
        }
    }

    // NOTE: if you modify this function, also modify the order in which we return
    // attributes in the get_attribute_list_mut() and get_attribute_list() functions!
    pub fn default_matrix() -> Self {
        NodeContents::Matrix {
            interval: 0,
            row_1: 1,
            row_2: 2,
            row_3: 3,
            output: 4,
        }
    }

    // NOTE: if you modify this function, also modify the order in which we return
    // attributes in the get_attribute_list_mut() and get_attribute_list() functions!
    pub fn default_rotation_matrix() -> Self {
        NodeContents::RotationMatrix {
            axis: 0,
            angle: 1,
            output: 2,
        }
    }

    // NOTE: if you modify this function, also modify the order in which we return
    // attributes in the get_attribute_list_mut() and get_attribute_list() functions!
    pub fn default_translation_matrix() -> Self {
        NodeContents::TranslationMatrix {
            vector: 0,
            output: 1,
        }
    }

    // NOTE: if you modify this function, also modify the order in which we return
    // attributes in the get_attribute_list_mut() and get_attribute_list() functions!
    pub fn default_transform() -> Self {
        NodeContents::Transform {
            geometry: 0,
            matrix: 1,
            output: 2,
        }
    }

    // NOTE: if you modify this function, also modify the order in which we return
    // attributes in the get_attribute_list_mut() and get_attribute_list() functions!
    pub fn default_rendering() -> Self {
        NodeContents::Rendering {
            geometry: 0,
            thickness: 1,
            mask: 2,
            material: 3,
        }
    }

    // NOTE: if you modify this function, also modify the order in which we return
    // attributes in the get_attribute_list_mut() and get_attribute_list() functions!
    pub fn default_vector_rendering() -> Self {
        NodeContents::VectorRendering {
            application_point: 0,
            vector: 1,
            thickness: 2,
            material: 3,
        }
    }

    // NOTE: if you modify this function, also modify the order in which we return
    // attributes in the get_attribute_list_mut() and get_attribute_list() functions!
    pub fn default_primitive() -> Self {
        NodeContents::Primitive {
            primitive: 0,
            size: 1,
            output: 2,
        }
    }
}


#[derive(Clone, Deserialize, Serialize, Debug,)]
pub struct Node {
    pub title: String,
    position: [f32; 2],
    error: Option<GraphError>,
    pub contents: NodeContents, // TODO: made this public to implement the translation matrix more easily. change back to private
}

impl Node {
    pub fn contents(&self) -> &NodeContents {
        &self.contents
    }

    pub fn render(&mut self, ui: &imgui::Ui<'_>, availables: &Availables, attributes: &mut Vec<Option<Attribute>>) -> bool {
        imnodes::BeginNodeTitleBar();
            ui.text(&self.title);
            // handle error reporting
            if let Some(error) = &self.error {
                ui.same_line();
                match error.severity {
                    Severity::Warning => {
                        ui.text_colored( [1.0, 0.8, 0.0, 1.0], "⚠");
                    },
                    Severity::Error => {
                        ui.text_colored( [1.0, 0.8, 0.0, 1.0], "⊗");
                    }
                }
                if ui.is_item_hovered() {
                    ui.tooltip_text(&error.message);
                }
            }
        imnodes::EndNodeTitleBar();
        // TODO: not sure if we will be able to use the get_attribute_list()
        // when we introduce the Group kind node in the future...
        Attribute::render_list(ui, availables, attributes, self.contents.get_attribute_list())
    }

    pub fn get_input_nodes(&self, graph: &NodeGraph) -> Vec::<NodeID> {
        self.contents.get_attribute_list()
            .into_iter()
            .filter_map(|attribute_id| {
                // this function will return None if the attribute is not an InputPin,
                // or if the InputPin is not connected to anything, so we can just
                // feed it the entire list of attributes.
                graph.get_attribute_as_linked_node(attribute_id)
            })
            .collect()
    }

    pub fn get_owned_attributes_mut(&mut self) -> Vec::<&mut AttributeID> {
        self.contents.get_attribute_list_mut()
    }

    pub fn get_owned_attributes(&self) -> Vec::<AttributeID> {
        self.contents.get_attribute_list()
    }
}

#[derive(Clone, Deserialize, Serialize, Debug,)]
pub enum Severity {
    Warning,
    Error
}
#[derive(Clone, Deserialize, Serialize, Debug,)]
pub struct GraphError {
    pub node_id: NodeID,
    pub severity: Severity,
    pub message: String,
}

#[derive(Clone, Deserialize, Serialize, Debug,)]
pub struct NodeGraph {
    nodes: Vec<Option<Node>>,
    attributes: Vec<Option<Attribute>>,
    links: HashMap::<AttributeID, AttributeID>,
    free_nodes_list: Vec<NodeID>,
    free_attributes_list: Vec<AttributeID>,
    #[serde(skip)]
    pub zoom_level: usize,
    // TODO: review this, having an option makes very little sense because the only
    // time we change it is when we right click on something, and the only time
    // we use the information is inside popups that come right after.
    // Having "dangling" ids should never be a problem at all, while zeroing
    // the option is extremely complicated due to the rename popup.
    #[serde(skip)]
    right_clicked_node: Option<NodeID>,
    #[serde(skip)]
    right_clicked_link: Option<AttributeID>,
    #[serde(skip)]
    editing_node: Option<NodeID>,
    #[serde(skip)]
    last_edit_timestamp: f64,
}

enum PairInfo {
    FirstInputSecondOutput,
    FirstOutputSecondInput,
    NonCompatible
}

impl Default for NodeGraph {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            attributes: Vec::new(),
            links: HashMap::new(),
            free_nodes_list: Vec::new(),
            free_attributes_list: Vec::new(),
            right_clicked_node: None,
            right_clicked_link: None,
            editing_node: None,
            zoom_level: 0,
            last_edit_timestamp: 0.0,
        }
    }
}

impl NodeGraph {
    pub fn currently_editing(&self) -> bool {
        self.editing_node.is_some()
    }

    pub fn stop_editing(&mut self) {
        self.editing_node = None;
    }

    fn get_new_node_id(&mut self) -> NodeID {
        if let Some(id) = self.free_nodes_list.pop() {
            // if there is any free slot in the nodes, then use that slot
            id
        } else {
            // otherwise, push a new, empty slot onto the nodes vec and use that one
            let id = self.nodes.len() as NodeID;
            self.nodes.push(None);
            id
        }
    }

    fn get_new_attribute_id(&mut self) -> AttributeID {
        if let Some(id) = self.free_attributes_list.pop() {
            // if there is any free slot in the nodes, then use that slot
            id
        } else {
            // otherwise, push a new, empty slot onto the nodes vec and use that one
            let id = self.attributes.len() as NodeID;
            self.attributes.push(None);
            id
        }
    }

    pub fn insert_node(&mut self, title: String, position: [f32; 2], node_contents: NodeContents, attributes_contents: Vec<AttributeContents>) -> NodeID {
        let mut node = Node {
            title,
            position,
            error: None,
            contents: node_contents
        };
        // make a check: the list of owned attributes must have the same
        // length as the attributes vector
        let owned_attributes = node.get_owned_attributes_mut();
        assert!(owned_attributes.len() == attributes_contents.len());
        // first, get an id for the to-be-inserted node
        let node_id = self.get_new_node_id();

        // we now need to insert all the attributes in our graph.
        // This is tricky because we need to remap the AttributeContents indices
        // to the new ids that the attributes will have!
        let mut new_id_map = Vec::<AttributeID>::new();
        for contents in attributes_contents.into_iter() {
            let attribute_id = self.get_new_attribute_id();
            // store the new attribute id in our map
            new_id_map.push(attribute_id);
            // and push the attribute to that location. Also check that we are not overwriting
            // some existing attribute, that would mean something is off!
            assert!(self.attributes[attribute_id as usize].is_none());
            self.attributes[attribute_id as usize] = Some(Attribute{ node_id, contents });
        }

        // now, before pushing our node to the graph, we need to modify all the attribute_ids it
        // contains!
        // into iter returns a reference to each attribute stored in our node.
        // we take the original attribute id, pass it through our map and then
        // overwrite the content of the reference (i.e: the integer contained
        // inside our node structure) with the mapped output.
        for id_reference in owned_attributes.into_iter() {
            *id_reference = new_id_map[*id_reference as usize];
        }
        // we can finally push the node. Also check that we are not overwriting
        // some existing attribute, that would mean something is off!
        assert!(self.nodes[node_id as usize].is_none());
        self.nodes[node_id as usize] = Some(node);
        node_id
    }

    pub fn push_all_to_corner(&mut self) {
        let mut min_left = std::f32::MAX;
        let mut min_up = std::f32::MAX;
        for maybe_node in self.nodes.iter() {
            if let Some(node) = maybe_node.as_ref() {
                let [pos_x, pos_y] = node.position;
                min_left = min_left.min(pos_x);
                min_up = min_up.min(pos_y);
            }
        }

        for maybe_node in self.nodes.iter_mut() {
            if let Some(node) = maybe_node.as_mut() {
                let [pos_x, pos_y] = node.position;
                node.position = [pos_x - min_left, pos_y - min_up];
            }
        }
    }

    pub fn push_positions_to_imnodes(&self) {
        for (idx, maybe_node) in self.nodes.iter().enumerate() {
            if let Some(node) = maybe_node.as_ref() {
                let [pos_x, pos_y] = node.position;
                let zoom = ZOOM_LEVELS[self.zoom_level];
                let editor_pos = [pos_x*zoom, pos_y*zoom];
                imnodes::SetNodePosition(idx as NodeID, editor_pos);
            }
        }
    }

    pub fn read_positions_from_imnodes(&mut self) {
        for (idx, maybe_node) in self.nodes.iter_mut().enumerate() {
            if let Some(node) = maybe_node.as_mut() {
                let [pos_x, pos_y] = imnodes::GetNodePosition(idx as NodeID);
                let zoom = ZOOM_LEVELS[self.zoom_level];
                node.position = [pos_x/zoom, pos_y/zoom];
            }
        }
    }

    pub fn zoom_down_graph(&mut self, mouse_pos: [f32; 2]) {
        if self.zoom_level < ZOOM_LEVELS.len() - 1 {
            let prev_zoom = ZOOM_LEVELS[self.zoom_level];
            self.zoom_level += 1;
            let new_zoom = ZOOM_LEVELS[self.zoom_level];
            let [mouse_x, mouse_y] = mouse_pos;
            let [pan_x, pan_y] = imnodes::GetEditorPanning();
            let new_x = (pan_x-mouse_x)*new_zoom/prev_zoom + mouse_x;
            let new_y = (pan_y-mouse_y)*new_zoom/prev_zoom + mouse_y;
            imnodes::SetEditorPanning([new_x, new_y]);
            self.push_positions_to_imnodes();
        }
    }

    pub fn zoom_up_graph(&mut self, mouse_pos: [f32; 2]) {
        if self.zoom_level > 0 {
            let prev_zoom = ZOOM_LEVELS[self.zoom_level];
            self.zoom_level -= 1;
            let new_zoom = ZOOM_LEVELS[self.zoom_level];
            let [mouse_x, mouse_y] = mouse_pos;
            let [pan_x, pan_y] = imnodes::GetEditorPanning();
            let new_x = (pan_x-mouse_x)*new_zoom/prev_zoom + mouse_x;
            let new_y = (pan_y-mouse_y)*new_zoom/prev_zoom + mouse_y;
            imnodes::SetEditorPanning([new_x, new_y]);
            self.push_positions_to_imnodes();
        }
    }

    pub fn render(&mut self, ui: &imgui::Ui<'_>, availables: &Availables, graph_fonts: &[imgui::FontId]) -> Option<f64> {
        let mut request_savestate: Option<f64> = None;
        let graph_font = graph_fonts[self.zoom_level];
        let style_shim = create_style_shim(ZOOM_LEVELS[self.zoom_level]);
        imnodes::ApplyStyle(&style_shim);
        let font_token = ui.push_font(graph_font);
        let editor_ne_point = ui.cursor_pos();
        self.push_positions_to_imnodes();
        imnodes::BeginNodeEditor();
        // render all links first. Remember that link ID is the same as the input attribute ID!
        for pair in self.links.iter() {
            let link_id = pair.0;
            let input_attribute_id = pair.0;
            let output_attribute_id = pair.1;
            imnodes::Link(*link_id, *input_attribute_id, *output_attribute_id);
        }

        // render all nodes
        for (idx, maybe_node) in self.nodes.iter_mut().enumerate() {
            if let Some(node) = maybe_node.as_mut() {
                imnodes::BeginNode(idx as NodeID);
                if node.render(ui, availables, &mut self.attributes) {
                    self.last_edit_timestamp = ui.time();
                }
                imnodes::EndNode();
            }
        }

        // we can only check if the editor is hovered before we call EndNodeEditor()
        let editor_hovered = imnodes::IsEditorHovered();

        // on the contrary, we need to end the node editor before doing any interaction
        // (e.g: right clicks, node creation)
        imnodes::EndNodeEditor();
        self.read_positions_from_imnodes();
        font_token.pop();

        // Process right click
        let mouse_delta = ui.mouse_drag_delta_with_threshold(MouseButton::Right, 4.0);
        let right_click_popup: bool = ui.is_window_focused_with_flags(WindowFocusedFlags::ROOT_AND_CHILD_WINDOWS)
            && editor_hovered
            && !ui.is_any_item_hovered()
            && ui.is_mouse_released(MouseButton::Right)
            && mouse_delta == [0.0, 0.0]; // exact comparison is fine due to GetMouseDragDelta threshold

        let mut selected_nodes_ids = imnodes::GetSelectedNodes();
        if right_click_popup {
            let mut hovered_id: i32 = -1;
            if imnodes::IsNodeHovered(&mut hovered_id) {
                // Right-clicking on a node does not select it. This means that if a user right clicks
                // on a node that is not selected the interaction will be very confusing.
                // To make sure everything will be fine, we clear node selection if this is the case.
                if !selected_nodes_ids.contains(&hovered_id) {
                    imnodes::ClearNodeSelection();
                    selected_nodes_ids.clear();
                }

                self.right_clicked_node = Some(hovered_id);
                ui.open_popup("Node menu");
            } else if imnodes::IsLinkHovered(&mut hovered_id) {
                self.right_clicked_link = Some(hovered_id);
                ui.open_popup("Link menu");
            } else {
                ui.open_popup("Add menu");
            }
        }

        let mut workaround_open_rename = false;
        ui.popup("Node menu", || {
            let clicked_node = self.right_clicked_node.unwrap();
            // The right-click menu changes contents depending on how many nodes are selected
            if selected_nodes_ids.len() <= 1 {
                // single node selection, using the self.right_clicked_node id
                if MenuItem::new("delete node").build(ui) {
                    println!("need to remove {}", clicked_node);
                    self.remove_node(clicked_node);
                    imnodes::ClearNodeSelection();
                    self.right_clicked_node = None;
                    request_savestate = Some(ui.time());
                }
                // TODO: decide if single node clone should still clone the links
                if MenuItem::new("duplicate node").build(ui) {
                    self.duplicate_node_no_links(clicked_node);
                    self.right_clicked_node = None;
                    request_savestate = Some(ui.time());
                }
                if MenuItem::new("rename node").build(ui) {
                    workaround_open_rename = true;
                }
            } else {
                // multiple node selection, operates on all selected nodes
                if MenuItem::new("delete selected nodes").build(ui) {
                    for node_id in selected_nodes_ids.iter() {
                        println!("need to remove[] {}", *node_id);
                        self.remove_node(*node_id);
                    }
                    imnodes::ClearNodeSelection();
                    self.right_clicked_node = None;
                    request_savestate = Some(ui.time());
                }
                if MenuItem::new("duplicate nodes and links").build(ui) {
                    self.duplicate_nodes(&selected_nodes_ids);
                    self.right_clicked_node = None;
                    request_savestate = Some(ui.time());
                }
            }
        });

        if workaround_open_rename {
            ui.open_popup("Edit node name");
        }
        ui.popup("Edit node name", || {
            let mut string = String::new();
            let value_changed = InputText::new(ui, "", &mut string)
                .no_undo_redo(true)
                .enter_returns_true(true)
                .build();
            if value_changed {
                let node_id = self.right_clicked_node.unwrap();
                self.right_clicked_node = None;
                self.get_node_mut(node_id).unwrap().title = string.to_string();
                ui.close_current_popup();
            }
        });

        ui.popup("Link menu", || {
            let clicked_link = self.right_clicked_link.unwrap();
            if MenuItem::new("delete link").build(ui) {
                println!("need to remove {}", clicked_link);
                self.links.remove(&clicked_link);
                imnodes::ClearLinkSelection();
                self.right_clicked_link = None;
                request_savestate = Some(ui.time());
            }
        });

        let style_token = ui.push_style_var(StyleVar::WindowPadding([8.0, 8.0]));
        ui.popup("Add menu", || {
            let [click_pos_x, click_pos_y] = ui.mouse_pos_on_opening_current_popup();
            let [pan_x, pan_y] = imnodes::GetEditorPanning();
            let editor_pos_x = click_pos_x - editor_ne_point[0] - pan_x;
            let editor_pos_y = click_pos_y - editor_ne_point[1] - pan_y;
            let zoom = ZOOM_LEVELS[self.zoom_level];
            let node_pos = [editor_pos_x/zoom, editor_pos_y/zoom];

            ui.menu("Geometries", || {
                if MenuItem::new("Curve").build(ui) {
                    self.add_curve_node(node_pos);
                    request_savestate = Some(ui.time());
                }
                if MenuItem::new("Bezier Curve").build(ui) {
                    self.add_bezier_node(node_pos);
                    request_savestate = Some(ui.time());
                }
                if MenuItem::new("Surface").build(ui) {
                    self.add_surface_node(node_pos);
                    request_savestate = Some(ui.time());
                }
                if MenuItem::new("Plane").build(ui) {
                    self.add_plane_node(node_pos);
                    request_savestate = Some(ui.time());
                }
                if MenuItem::new("Primitive").build(ui) {
                    self.add_primitive_node(node_pos);
                    request_savestate = Some(ui.time());
                }
            }); // Geometries menu ends here

            ui.menu("Parameters", || {
                if MenuItem::new("Interval").build(ui) {
                    self.add_interval_node(node_pos);
                    request_savestate = Some(ui.time());
                }
                if MenuItem::new("Sample parameter").build(ui) {
                    self.add_parameter_node(node_pos);
                    request_savestate = Some(ui.time());
                }
            }); // Geometries menu ends here

            ui.menu("Transformations", || {
                if MenuItem::new("Generic Matrix").build(ui) {
                    self.add_matrix_node(node_pos);
                    request_savestate = Some(ui.time());
                }
                if MenuItem::new("Rotation Matrix").build(ui) {
                    self.add_rotation_matrix_node(node_pos);
                    request_savestate = Some(ui.time());
                }
                if MenuItem::new("Translation Matrix").build(ui) {
                    self.add_translation_matrix_node(node_pos);
                    request_savestate = Some(ui.time());
                }
                if MenuItem::new("Transform").build(ui) {
                    self.add_transform_node(node_pos);
                    request_savestate = Some(ui.time());
                }
            }); // Transformations menu ends here

            if MenuItem::new("Point").build(ui) {
                self.add_point_node(node_pos);
                request_savestate = Some(ui.time());
            }
            if MenuItem::new("Vector").build(ui) {
                self.add_vector_node(node_pos);
                request_savestate = Some(ui.time());
            }
            if MenuItem::new("Geometry Rendering").build(ui) {
                self.add_rendering_node(node_pos);
                request_savestate = Some(ui.time());
            }
            if MenuItem::new("Vector Rendering").build(ui) {
                self.add_vector_rendering_node(node_pos);
                request_savestate = Some(ui.time());
            }
        }); // "Add" closure ends here
        style_token.pop();

        // check if a link was created
        let mut start_attribute_id: AttributeID = -1;
        let mut end_attribute_id: AttributeID = -1;
        if imnodes::IsLinkCreated(&mut start_attribute_id, &mut end_attribute_id) {
            let maybe_link = Self::check_link_creation(&self.attributes, start_attribute_id, end_attribute_id);
            // check which one of the two attributes is the input attribute and which is the output
            if let Some((input_id, output_id)) = maybe_link {
                self.links.insert(input_id, output_id);
                request_savestate = Some(ui.time());
            }
        }

        // check if we are actively editing a node or not.
        let mut active_attribute = 0;
        if imnodes::IsAnyAttributeActive(&mut active_attribute) {
            let attribute_slot = self.attributes.get(active_attribute as usize).unwrap();
            let editing_node_id = attribute_slot.as_ref().unwrap().node_id;
            match self.editing_node {
                None => {
                    // started editing a new node
                    self.editing_node = Some(editing_node_id);
                }
                Some(old_id) if old_id != editing_node_id => {
                    // stopped editing a node, started editing a new one
                    self.editing_node = Some(editing_node_id);
                    request_savestate = Some(self.last_edit_timestamp);
                }
                Some(_old_id) => { // if old_id == editing_node_id
                    // we are still editing the same node, do nothing
                }
            }
        } else {
            match self.editing_node {
                None => {
                    // we are still not editing any node, do nothing
                }
                Some(_old_id) => {
                    // stopped editing a node
                    self.editing_node = None;
                    request_savestate = Some(self.last_edit_timestamp);
                }
            }
        }

        request_savestate
    }

    pub fn get_nodes(&self) -> impl Iterator<Item = (NodeID, &Node)> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|pair| {
                if let Some(node) = pair.1.as_ref() {
                    Some((pair.0 as NodeID, node))
                } else {
                    None
                }
            })
    }

    pub fn get_nodes_mut(&mut self) -> impl Iterator<Item = (NodeID, &mut Node)> {
        self.nodes
            .iter_mut()
            .enumerate()
            .filter_map(|pair| {
                if let Some(node) = pair.1.as_mut() {
                    Some((pair.0 as NodeID, node))
                } else {
                    None
                }
            })
    }

    pub fn get_node(&self, id: NodeID) -> Option<&Node> {
        if let Some(maybe_node) = self.nodes.get(id as usize) {
            maybe_node.as_ref()
        } else {
            None
        }
    }

    pub fn get_node_mut(&mut self, id: NodeID) -> Option<&mut Node> {
        if let Some(maybe_node) = self.nodes.get_mut(id as usize) {
            maybe_node.as_mut()
        } else {
            None
        }
    }

    pub fn mark_error(&mut self, error: GraphError) {
         if let Some(Some(node)) = self.nodes.get_mut(error.node_id as usize) {
            node.error = Some(error);
         }
    }

    pub fn clear_all_errors(&mut self) {
        for slot in self.nodes.iter_mut() {
            if let Some(node) = slot {
                node.error = None;
            }
        }
    }

    // "clone" a node means "get all the data that we might need to insert a copy
    // of this node into the graph". We do NOT insert it in the graph.
    pub fn clone_node(&self, node_id: NodeID) -> Option<(String, [f32; 2], NodeContents, Vec<AttributeContents>)> {
        let node = self.get_node(node_id)?;
        let attributes_list = node.get_owned_attributes();

        let title = node.title.clone();
        let position = node.position;
        let cloned_contents = node.contents.default_same_kind();
        let cloned_attributes = attributes_list
            .into_iter()
            .filter_map(|id| -> Option<AttributeContents> {
                Some(
                    self.attributes.get(id as usize)?
                    .as_ref()?
                    .contents.clone()
                )
            })
            .collect();

        Some((title, position, cloned_contents, cloned_attributes))
    }

    fn duplicate_node_no_links(&mut self, node_id: NodeID) -> NodeID {
        let (title, orig_pos, cloned_contents, cloned_attributes) = self.clone_node(node_id).unwrap();
        let position = [orig_pos[0] + 40.0, orig_pos[1] + 40.0];
        self.insert_node(title, position, cloned_contents, cloned_attributes)
    }

    fn duplicate_nodes(&mut self, nodes_ids: &[NodeID]) {
        let mut original_to_cloned_id = std::collections::BTreeMap::<AttributeID, AttributeID>::new();
        let mut linked_inputs_list = Vec::<AttributeID>::new();
        for original_node_id in nodes_ids.iter() {
            // if there is no such a node, just continue
            if self.nodes.get(*original_node_id as usize).is_none() {
                continue;
            }

            // clone the node, insert it in the graph seting its position of the cloned node
            // to be the same as the original one, plus a little delta.
            let (title, orig_pos, cloned_node, cloned_attributes) = self.clone_node(*original_node_id).unwrap();
            let position = [orig_pos[0] + 40.0, orig_pos[1] + 120.0];
            let cloned_node_id = self.insert_node(title, position, cloned_node, cloned_attributes);

            // Get the list of owned attributes. Node that due to the way the list is generated,
            // the order in which the attributes will appear is the same for the original and the clone.
            let original_attributes_id = self.get_node(*original_node_id).unwrap().get_owned_attributes();
            let cloned_attributes_id = self.get_node(cloned_node_id).unwrap().get_owned_attributes();
            // now go through all the pairs of cloned-original attributes, add them to the map.
            // Also, if the original was linked to something, add it to the list of "needs to be linked in the end".
            let zipped_iterator = original_attributes_id.into_iter().zip(cloned_attributes_id.into_iter());
            for pair in zipped_iterator {
                original_to_cloned_id.insert(pair.0, pair.1);
                if self.links.contains_key(&pair.0) {
                    linked_inputs_list.push(pair.0);
                }
            }

        }

        // after this loop ends, we have cloned all the nodes that we wanted to clone, now we need
        // to add all the links. Go through the "needs to be linked in the end" and process it
        for original_input_id in linked_inputs_list.into_iter() {
            let cloned_input_id : i32 = *original_to_cloned_id.get(&original_input_id).unwrap();
            let original_output_id : i32 = *self.links.get(&original_input_id).unwrap();
            // if the original was cloned, then we want to link to the cloned one,
            // otherwise, link to the original one
            if let Some(cloned_output_id) = original_to_cloned_id.get(&original_output_id) {
                self.links.insert(cloned_input_id, *cloned_output_id);
            } else {
                self.links.insert(cloned_input_id, original_output_id);
            }
        }
    }

    fn remove_node(&mut self, node_id: NodeID) {
        // try to remove this node_id from the map
        if let Some(slot) = self.nodes.get_mut(node_id as usize) {
            // slot is a reference to the option! By taking() it, we effectively
            // remove the old node from the graph's nodes
            let old_node = slot.take().unwrap(); // this unwrap asserts that the old node is Some(thing)
            self.free_nodes_list.push(node_id);
            // if the node exists, get a list of all the attributes belonging to it
            // remove all the attributes of the attributes map.
            let list_of_attributes: Vec<AttributeID> = old_node.get_owned_attributes();

            // remove the attributes from our vector by marking the spot as None
            // and pushing that id to the free slots list
            for id in list_of_attributes.iter() {
                self.attributes[*id as usize] = None;
                self.free_attributes_list.push(*id);
            }

            // remove all the inbound AND outbound links.
            // the quickest way of doing it is just by rebuilding the link map
            // we do that by draining the map, filtering and collecting() it back.
            self.links = self.links
                .drain()
                .filter(|pair| {
                    !list_of_attributes.contains(&pair.0) && !list_of_attributes.contains(&pair.1)
                })
                .collect();
        }
    }

    pub fn get_attribute_as_usize(&self, attribute_id: AttributeID) -> Option<usize> {
        // first, we need to check if the attribute_id actually exists in our attributes map
        let attribute_slot = self.attributes.get(attribute_id as usize)?;
        // then, if the slot is here, we need to check if something is in the slot.
        let attribute = attribute_slot.as_ref()?;
        // finally, if the attribute exists, we need to check if we can convert it is a usize
        match attribute.contents {
            AttributeContents::IntSlider{ value, .. } => Some(value as usize),
            AttributeContents::PrimitiveKind{ selected } => Some(selected),
            AttributeContents::Mask{ selected } => Some(selected),
            AttributeContents::Material{ selected } => Some(selected),
            _ => None
        }
    }

    #[allow(unused)]
    pub fn get_attribute_as_color(&self, attribute_id: AttributeID) -> Option<[f32; 3]> {
        // first, we need to check if the attribute_id actually exists in our attributes map
        let attribute_slot = self.attributes.get(attribute_id as usize)?;
        // then, if the slot is here, we need to check if something is in the slot.
        let attribute = attribute_slot.as_ref()?;
        // finally, if the attribute exists, we need to check if it is a Color
        if let AttributeContents::Color{ color, .. } = attribute.contents {
            Some(color)
        } else {
            None
        }
    }

    pub fn get_attribute_as_string(&self, attribute_id: AttributeID) -> Option<String> {
        // first, we need to check if the attribute_id actually exists in our attributes map
        let attribute_slot = self.attributes.get(attribute_id as usize)?;
        // then, if the slot is here, we need to check if something is in the slot.
        let attribute = attribute_slot.as_ref()?;
        // finally, if the attribute exists, we need to check if it is a Text one
        if let AttributeContents::Text{ string, .. } = &attribute.contents {
            Some(string.to_string())
        } else {
            None
        }
    }

    pub fn get_attribute_as_matrix_row(&self, attribute_id: AttributeID) -> Option<[String; 4]> {
        // first, we need to check if the attribute_id actually exists in our attributes map
        let attribute_slot = self.attributes.get(attribute_id as usize)?;
        // then, if the slot is here, we need to check if something is in the slot.
        let attribute = attribute_slot.as_ref()?;
        // if it exists, then we need to check if it is a MatrixRow attribute.
        if let AttributeContents::MatrixRow{ col_1, col_2, col_3, col_4, } = &attribute.contents {
            Some([
                col_1.to_string(),
                col_2.to_string(),
                col_3.to_string(),
                col_4.to_string(),
            ])
        } else {
            None
        }
    }

    pub fn get_attribute_as_axis(&self, attribute_id: AttributeID) -> Option<Axis> {
        // first, we need to check if the attribute_id actually exists in our attributes map
        let attribute_slot = self.attributes.get(attribute_id as usize)?;
        // then, if the slot is here, we need to check if something is in the slot.
        let attribute = attribute_slot.as_ref()?;
        // if it exists, then we need to check if it is a MatrixRow attribute.
        if let AttributeContents::AxisSelect{ axis } = attribute.contents {
            Some(axis)
        } else {
            None
        }
    }

    pub fn get_attribute_as_linked_output(&self, attribute_id: AttributeID) -> Option<AttributeID> {
        // first, we need to check if the attribute_id actually exists in our attributes map
        let attribute_slot = self.attributes.get(attribute_id as usize)?;
        // then, if the slot is here, we need to check if something is in the slot.
        let attribute = attribute_slot.as_ref()?;
        // if it exists, then we need to check if it is an input pin
        if let AttributeContents::InputPin{..} = &attribute.contents {
            // and then we can finally check if it is actually linked to something!
            let linked_output_id = self.links.get(&attribute_id)?;
            Some(*linked_output_id)
        } else {
            None
        }
    }

    pub fn get_attribute_as_linked_node(&self, input_attribute_id: AttributeID) -> Option<NodeID> {
        // this could probably be written as a option::and_then() or something similar
        if let Some(output_attribute_id) = self.get_attribute_as_linked_output(input_attribute_id) {
            let linked_node_id = self.attributes[output_attribute_id as usize].as_ref().unwrap().node_id;
            Some(linked_node_id)
        } else {
            None
        }
    }

    fn check_pair_info(first_attribute: &Attribute, second_attribute: &Attribute) -> PairInfo {
        // sort them and match them based on the first content
        // this match is probably over complicated and might be reworked to be easier to follow.
        // Also, when we re-introduce the idea of a "value" as a possible input to a matrix, we will have to change it for sure.
        // TODO: there is a lot of rightwards drift in this match, perhaps early returns are better?
        match &first_attribute.contents {
            AttributeContents::InputPin { kind: first_kind, .. } => {
                if let AttributeContents::OutputPin { kind: ref second_kind, .. } = second_attribute.contents {
                    // note: rust automatically derefs when doing comparisons!
                    if first_kind == second_kind {
                        PairInfo::FirstInputSecondOutput
                    } else {
                        PairInfo::NonCompatible
                    }
                } else {
                    PairInfo::NonCompatible
                }
            },
            AttributeContents::OutputPin { kind: first_kind, .. } => {
                if let AttributeContents::InputPin { kind: ref second_kind, .. } = second_attribute.contents {
                    // note: rust automatically derefs when doing comparisons!
                    if first_kind == second_kind {
                        PairInfo::FirstOutputSecondInput
                    } else {
                        PairInfo::NonCompatible
                    }
                } else {
                    PairInfo::NonCompatible
                }
            },
            _ => PairInfo::NonCompatible
        }
    }

    // checks if the two pins are compatible and if they are, return a sorted pair:
    // the first id is the one belonging to the input attribute.
    fn check_link_creation(attributes: &[Option<Attribute>], first_id: AttributeID, second_id: AttributeID) -> Option<(AttributeID, AttributeID)> {
        let first_attribute_opt = attributes.get(first_id as usize);
        let second_attribute_opt = attributes.get(second_id as usize);
        match (first_attribute_opt, second_attribute_opt) {
            // if both attributes actually exist, check if they are compatible
            (Some(Some(first_attribute)), Some(Some(second_attribute))) => {
                let pair_info = Self::check_pair_info(first_attribute, second_attribute);
                match pair_info {
                    PairInfo::FirstInputSecondOutput => Some((first_id, second_id)),
                    PairInfo::FirstOutputSecondInput => Some((second_id, first_id)),
                    PairInfo::NonCompatible => None,
                }
            },
            // TODO: maybe log a warning instead of panic?
            (Some(None), Some(_)) => unreachable!("When attempting to create a link, the first attribute was not found in the map"),
            (None, Some(_)) => unreachable!("When attempting to create a link, the first attribute was not found in the map"),
            (Some(_), Some(None)) => unreachable!("When attempting to create a link, the second attribute was not found in the map"),
            (Some(_), None) => unreachable!("When attempting to create a link, the second attribute was not found in the map"),
            (None, None) => unreachable!("When attempting to create a link, none of the two attributes was found in the map"),
        }
    }

    pub fn add_interval_node(&mut self, position: [f32; 2]) -> NodeID {
        // NOTE: the order here is important: the attributes here
        // must appear in the same order as they do in the default_interval() function!
        let attributes_contents = vec![
            AttributeContents::Text {
                label: String::from(" name"),
                string: String::from(""),
            },
            AttributeContents::Text {
                label: String::from("begin"),
                string: String::from(""),
            },
            AttributeContents::Text {
                label: String::from("  end"),
                string: String::from(""),
            },
            AttributeContents::IntSlider {
                label: String::from("quality"),
                value: 4,
                mode: SliderMode::IntRange(1, 16),
            },
            AttributeContents::OutputPin {
                label: String::from("interval"),
                kind: DataKind::Interval,
            }
        ];
        let node_contents = NodeContents::default_interval();
        self.insert_node("Interval".into(), position, node_contents, attributes_contents)
    }

    pub fn add_parameter_node(&mut self, position: [f32; 2]) -> NodeID {
        // NOTE: the order here is important: the attributes here
        // must appear in the same order as they do in the default_interval() function!
        let attributes_contents = vec![
            AttributeContents::InputPin {
                label: String::from("geometry"),
                kind: DataKind::Geometry,
            },
            AttributeContents::Text {
                label: String::from("param:"),
                string: String::from(""),
            },
            AttributeContents::Text {
                label: String::from("value:"),
                string: String::from(""),
            },
            AttributeContents::OutputPin {
                label: String::from("output"),
                kind: DataKind::Geometry,
            }
        ];
        let node_contents = NodeContents::default_sample();
        self.insert_node("Sample Parameter".into(), position, node_contents, attributes_contents)
    }

    pub fn add_vector_node(&mut self, position: [f32; 2]) -> NodeID {
        // NOTE: the order here is important: the attributes here
        // must appear in the same order as they do in the default_vector() function!
        let attributes_contents = vec![
            AttributeContents::Text {
                label: String::from("x"),
                string: String::from("0.0"),
            },
            AttributeContents::Text {
                label: String::from("y"),
                string: String::from("0.0"),
            },
            AttributeContents::Text {
                label: String::from("z"),
                string: String::from("0.0"),
            },
            AttributeContents::OutputPin {
                label: String::from("vector"),
                kind: DataKind::Vector,
            }
        ];
        let node_contents = NodeContents::default_vector();
        self.insert_node("Vector".into(), position, node_contents, attributes_contents)
    }

    pub fn add_point_node(&mut self, position: [f32; 2]) -> NodeID {
        // NOTE: the order here is important: the attributes here
        // must appear in the same order as they do in the default_point() function!
        let attributes_contents = vec![
            AttributeContents::Text {
                label: String::from("x"),
                string: String::from("0.0"),
            },
            AttributeContents::Text {
                label: String::from("y"),
                string: String::from("0.0"),
            },
            AttributeContents::Text {
                label: String::from("z"),
                string: String::from("0.0"),
            },
            AttributeContents::OutputPin {
                label: String::from("geometry"),
                kind: DataKind::Geometry,
            }
        ];
        let node_contents = NodeContents::default_point();
        self.insert_node("Point".into(), position, node_contents, attributes_contents)
    }

    pub fn add_bezier_node(&mut self, position: [f32; 2]) -> NodeID {
        // NOTE: the order here is important: the attributes here
        // must appear in the same order as they do in the default_bezier() function!
        let attributes_contents = vec![
            AttributeContents::InputPin {
                label: String::from("P0"),
                kind: DataKind::Geometry,
            },
            AttributeContents::InputPin {
                label: String::from("P1"),
                kind: DataKind::Geometry,
            },
            AttributeContents::InputPin {
                label: String::from("P2"),
                kind: DataKind::Geometry,
            },
            AttributeContents::InputPin {
                label: String::from("P3"),
                kind: DataKind::Geometry,
            },
            AttributeContents::IntSlider {
                label: String::from("quality"),
                value: 4,
                mode: SliderMode::IntRange(1, 16),
            },
            AttributeContents::OutputPin {
                label: String::from("geometry"),
                kind: DataKind::Geometry,
            }
        ];
        let node_contents = NodeContents::default_bezier();
        self.insert_node("Bézier".into(), position, node_contents, attributes_contents)
    }

    pub fn add_curve_node(&mut self, position: [f32; 2]) -> NodeID {
        // NOTE: the order here is important: the attributes here
        // must appear in the same order as they do in the default_curve() function!
        let attributes_contents = vec![
            AttributeContents::InputPin {
                label: String::from("interval"),
                kind: DataKind::Interval,
            },
            AttributeContents::Text {
                label: String::from("fx"),
                string: String::from(""),
            },
            AttributeContents::Text {
                label: String::from("fy"),
                string: String::from(""),
            },
            AttributeContents::Text {
                label: String::from("fz"),
                string: String::from(""),
            },
            AttributeContents::OutputPin {
                label: String::from("geometry"),
                kind: DataKind::Geometry,
            }
        ];
        let node_contents = NodeContents::default_curve();
        self.insert_node("Curve".into(), position, node_contents, attributes_contents)
    }

    pub fn add_surface_node(&mut self, position: [f32; 2]) -> NodeID {
        // NOTE: the order here is important: the attributes here
        // must appear in the same order as they do in the default_surface function!
        let attributes_contents = vec![
            AttributeContents::InputPin {
                label: String::from("interval 1"),
                kind: DataKind::Interval,
            },
            AttributeContents::InputPin {
                label: String::from("interval 2"),
                kind: DataKind::Interval,
            },
            AttributeContents::Text {
                label: String::from("fx"),
                string: String::from(""),
            },
            AttributeContents::Text {
                label: String::from("fy"),
                string: String::from(""),
            },
            AttributeContents::Text {
                label: String::from("fz"),
                string: String::from(""),
            },
            AttributeContents::OutputPin {
                label: String::from("geometry"),
                kind: DataKind::Geometry,
            }
        ];
        let node_contents = NodeContents::default_surface();
        self.insert_node("Surface".into(), position, node_contents, attributes_contents)
    }

    pub fn add_plane_node(&mut self, position: [f32; 2]) -> NodeID {
        let attributes_contents = vec![
            AttributeContents::InputPin {
                label: String::from("point"),
                kind: DataKind::Geometry,
            },
            AttributeContents::InputPin {
                label: String::from("normal"),
                kind: DataKind::Vector,
            },
            AttributeContents::IntSlider {
                label: String::from("size:"),
                value: 4,
                mode: SliderMode::IntRange(1, 16),
            },
            AttributeContents::OutputPin {
                label: String::from("output"),
                kind: DataKind::Geometry,
            }
        ];
        let node_contents = NodeContents::default_plane();
        self.insert_node("Plane".into(), position, node_contents, attributes_contents)
    }

    pub fn add_rendering_node(&mut self, position: [f32; 2]) -> NodeID {
        let attributes_contents = vec![
            AttributeContents::InputPin {
                label: String::from("geometry"),
                kind: DataKind::Geometry,
            },
            AttributeContents::IntSlider {
                label: String::from("thickness:"),
                value: 3,
                mode: SliderMode::SizeLabels,
            },
            AttributeContents::Mask {
                selected: 0,
            },
            AttributeContents::Material {
                selected: 0,
            },
        ];
        let node_contents = NodeContents::default_rendering();
        self.insert_node("Rendering".into(), position, node_contents, attributes_contents)
    }

    pub fn add_vector_rendering_node(&mut self, position: [f32; 2]) -> NodeID {
        let attributes_contents = vec![
            AttributeContents::InputPin {
                label: String::from("application point"),
                kind: DataKind::Geometry,
            },
            AttributeContents::InputPin {
                label: String::from("vector"),
                kind: DataKind::Vector,
            },
            AttributeContents::IntSlider {
                label: String::from("thickness:"),
                value: 3,
                mode: SliderMode::SizeLabels,
            },
            AttributeContents::Material {
                selected: 0,
            },
        ];
        let node_contents = NodeContents::default_vector_rendering();
        self.insert_node("Vector Rendering".into(), position, node_contents, attributes_contents)
    }

    pub fn add_primitive_node(&mut self, position: [f32; 2]) -> NodeID {
        let attributes_contents = vec![
            AttributeContents::PrimitiveKind {
                // due to alphabetical order of primitives, cube is selection number 1
                selected: 1,
            },
            AttributeContents::Text {
                label: String::from("size:"),
                string: String::from("1.0"),
            },
            AttributeContents::OutputPin {
                label: String::from("geometry"),
                kind: DataKind::Geometry,
            },
        ];
        let node_contents = NodeContents::default_primitive();
        self.insert_node("Primitive".into(), position, node_contents, attributes_contents)
    }

    pub fn add_transform_node(&mut self, position: [f32; 2]) -> NodeID {
        let attributes_contents = vec![
            AttributeContents::InputPin {
                label: String::from("geometry"),
                kind: DataKind::Geometry,
            },
            AttributeContents::InputPin {
                label: String::from("matrix"),
                kind: DataKind::Matrix,
            },
            AttributeContents::OutputPin {
                label: String::from("output"),
                kind: DataKind::Geometry,
            }
        ];
        let node_contents = NodeContents::default_transform();
        self.insert_node("Transform".into(), position, node_contents, attributes_contents)
    }

    pub fn add_matrix_node(&mut self, position: [f32; 2]) -> NodeID {
        let attributes_contents = vec![
            AttributeContents::InputPin {
                label: String::from("interval"),
                kind: DataKind::Interval,
            },
            AttributeContents::MatrixRow {
                col_1: "1.0".into(),
                col_2: "0.0".into(),
                col_3: "0.0".into(),
                col_4: "0.0".into(),
            },
            AttributeContents::MatrixRow {
                col_1: "0.0".into(),
                col_2: "1.0".into(),
                col_3: "0.0".into(),
                col_4: "0.0".into(),
            },
            AttributeContents::MatrixRow {
                col_1: "0.0".into(),
                col_2: "0.0".into(),
                col_3: "1.0".into(),
                col_4: "0.0".into(),
            },
            AttributeContents::OutputPin {
                label: String::from("output"),
                kind: DataKind::Matrix,
            },
        ];
        let node_contents = NodeContents::default_matrix();
        self.insert_node("Matrix".into(), position, node_contents, attributes_contents)
    }

    pub fn add_rotation_matrix_node(&mut self, position: [f32; 2]) -> NodeID {
        let attributes_contents = vec![
            AttributeContents::AxisSelect {
                axis: Axis::X,
            },
            AttributeContents::Text {
                label: String::from("angle"),
                string: String::from("0.0"),
            },
            AttributeContents::OutputPin {
                label: String::from("output"),
                kind: DataKind::Matrix,
            },
        ];
        let node_contents = NodeContents::default_rotation_matrix();
        self.insert_node("Rotation Matrix".into(), position, node_contents, attributes_contents)
    }

    pub fn add_translation_matrix_node(&mut self, position: [f32; 2]) -> NodeID {
        let attributes_contents = vec![
            AttributeContents::InputPin {
                label: String::from("vector"),
                kind: DataKind::Vector,
            },
            AttributeContents::OutputPin {
                label: String::from("output"),
                kind: DataKind::Matrix,
            },
        ];
        let node_contents = NodeContents::default_translation_matrix();
        self.insert_node("Translation Matrix".into(), position, node_contents, attributes_contents)
    }

}
