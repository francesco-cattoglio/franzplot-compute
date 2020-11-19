use std::collections::BTreeMap;
use crate::cpp_gui::imnodes;
use crate::cpp_gui::PinShape;
use imgui::*;

pub type AttributeID = i32;
pub type NodeID = i32;

#[derive(PartialEq)]
pub enum DataKind {
    Interval,
    Geometry,
    Matrix,
}

impl DataKind {
    // we might even return a color as well!
    fn to_pin_shape(&self) -> PinShape {
        match self {
            DataKind::Interval => PinShape::QuadFilled,
            DataKind::Geometry => PinShape::CircleFilled,
            DataKind::Matrix => PinShape::Quad,
        }
    }
}

pub struct Attribute {
    id: AttributeID,
    node_id: NodeID,
    contents: AttributeContents,
}

pub enum AttributeContents {
    InputPin {
        label: ImString,
        kind: DataKind,
    },
    OutputPin {
        label: ImString,
        kind: DataKind,
    },
    Text {
        label: ImString,
        string: ImString,
    },
    Unknown {
        label: ImString,
    }
}

impl Attribute {
    pub fn render(&mut self, ui: &imgui::Ui<'_>) {
        match &mut self.contents {
            AttributeContents::InputPin {
                label, kind,
            } => {
                imnodes::BeginInputAttribute(self.id, kind.to_pin_shape());
                ui.text(label);
                imnodes::EndInputAttribute();
            },
            AttributeContents::OutputPin {
                label, kind,
            } => {
                imnodes::BeginOutputAttribute(self.id, kind.to_pin_shape());
                ui.text(label);
                imnodes::EndOutputAttribute();
            },
            AttributeContents::Text{
                label, string,
            } => {
                imnodes::BeginStaticAttribute(self.id);
                ui.text(&label);
                ui.same_line(0.0);
                ui.set_next_item_width(80.0);
                InputText::new(ui, im_str!(""), string).build();
                imnodes::EndStaticAttribute();
            },
            _ => {
            }
        }
    }

    pub fn as_string(&self) -> String {
        match &self.contents {
            AttributeContents::Text{
                string, ..
            } => {
                string.to_string()
            }
            _ => panic!("wrong conversion: attribute to string")
        }
    }
}

pub struct Node {
    id: i32,
    title: ImString,
    error: Option<GraphError>,
    contents: NodeContents,
}

pub enum NodeContents {
    Interval {
        variable: AttributeID,
        begin: AttributeID,
        end: AttributeID,
        output: AttributeID,
    },
    Curve {
        interval: AttributeID,
        fx: AttributeID,
        fy: AttributeID,
        fz: AttributeID,
        output: AttributeID,
    },
    Surface,
    Transform,
    Matrix,
    Rendering {
        geometry: AttributeID,
    },
    Other
}

impl Node {
    pub fn contents(&self) -> &NodeContents {
        &self.contents
    }

    pub fn render(&mut self, ui: &imgui::Ui<'_>, attributes: &mut BTreeMap<AttributeID, Attribute>) {
        imnodes::BeginNode(self.id);
            imnodes::BeginNodeTitleBar();
                ui.text(&self.title);
                // handle error reporting
                if let Some(error) = &self.error {
                    ui.same_line(0.0);
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
            match &self.contents {
                NodeContents::Interval {
                    variable, begin, end, output,
                } => {
                    attributes.get_mut(output).unwrap().render(ui);
                    attributes.get_mut(variable).unwrap().render(ui);
                    attributes.get_mut(begin).unwrap().render(ui);
                    attributes.get_mut(end).unwrap().render(ui);
                },
                NodeContents::Curve {
                    interval, fx, fy, fz, output,
                } => {
                    attributes.get_mut(output).unwrap().render(ui);
                    attributes.get_mut(interval).unwrap().render(ui);
                    attributes.get_mut(fx).unwrap().render(ui);
                    attributes.get_mut(fy).unwrap().render(ui);
                    attributes.get_mut(fz).unwrap().render(ui);
                },
                NodeContents::Rendering {
                    geometry,
                } => {
                    attributes.get_mut(geometry).unwrap().render(ui);
                }
                _ => {}
            }
        imnodes::EndNode();
    }

    pub fn get_inputs(&self, graph: &NodeGraph) -> Vec::<Option<NodeID>> {
        match &self.contents {
            NodeContents::Interval { .. } => {
                vec![]
            },
            NodeContents::Curve { interval, .. } => {
                vec![graph.get_attribute_as_linked_node(*interval)]
            },
            NodeContents::Rendering { geometry, } => {
                vec![graph.get_attribute_as_linked_node(*geometry)]
            },
            _ => {
                vec![]
            }
        }
    }
}

pub enum Severity {
    Warning,
    Error
}
pub struct GraphError {
    pub node_id: NodeID,
    pub severity: Severity,
    pub message: ImString,
}

// TODO: get a constructor and make next_id private!
pub struct NodeGraph {
    pub nodes: BTreeMap::<NodeID, Node>,
    pub attributes: BTreeMap::<AttributeID, Attribute>,
    pub links: BTreeMap::<AttributeID, AttributeID>,
    pub next_id: i32,
}

impl NodeGraph {
    pub fn render(&mut self, ui: &imgui::Ui<'_>) {
        imnodes::BeginNodeEditor();
        // render all links first. Remember that link ID is the same as the input attribute ID!
        for pair in self.links.iter() {
            let link_id = pair.0;
            let input_attribute_id = pair.0;
            let output_attribute_id = pair.1;
            imnodes::Link(*link_id, *input_attribute_id, *output_attribute_id);
        }

        // render all nodes
        for node in self.nodes.values_mut() {
            node.render(ui, &mut self.attributes);
        }

        // we need to end the node editor before doing any interaction
        // (e.g: right clicks, node creation)
        imnodes::EndNodeEditor();

        // check if a link was created
        let mut start_attribute_id: AttributeID = 0;
        let mut end_attribute_id: AttributeID = 0;
        if imnodes::IsLinkCreated(&mut start_attribute_id, &mut end_attribute_id) {
            let maybe_link = Self::check_link_creation(&self.attributes, start_attribute_id, end_attribute_id);
            // check which one of the two attributes is the input attribute and which is the output
            if let Some((input_id, output_id)) = maybe_link {
                self.links.insert(input_id, output_id);
            }
        }
    }

    pub fn mark_error(&mut self, error: GraphError) {
         if let Some(node) = self.nodes.get_mut(&error.node_id) {
            node.error = Some(error);
         }
    }

    pub fn clear_all_errors(&mut self) {
        for node in self.nodes.values_mut() {
            node.error = None;
        }
    }

    pub fn get_attribute_as_string(&self, attribute_id: AttributeID) -> Option<String> {
        // first, we need to check if the attribute_id actually exists in our attributes map
        if let Some(attribute) = self.attributes.get(&attribute_id) {
            // if it exists, then we need to check if it is an input pin
            if let AttributeContents::Text{ string, .. } = &attribute.contents {
                Some(string.to_string())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_attribute_as_linked_node(&self, input_attribute_id: AttributeID) -> Option<NodeID> {
        // first, we need to check if the input_attribute_id actually exists in our attributes map
        if let Some(attribute) = self.attributes.get(&input_attribute_id) {
            // if it exists, then we need to check if it is an input pin
            if let AttributeContents::InputPin{..} = &attribute.contents {
                // and then we can finally check if it is actually linked to something!
                if let Some(output_attribute_id) = self.links.get(&attribute.id) {
                    // we can assert that if there is a link, then the linked one exists
                    let linked_node_id = self.attributes.get(output_attribute_id).unwrap().node_id;
                    Some(linked_node_id)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn check_pin_compatibility(first_attribute: &Attribute, second_attribute: &Attribute) -> Option<(AttributeID, AttributeID)> {
        // sort them and match them based on the first content
        // this match is probably over complicated and might be reworked to be easier to follow.
        // Also, when we re-introduce the idea of a "value" as a possible input to a matrix, we will have to change it for sure.
        match &first_attribute.contents {
            AttributeContents::InputPin { kind: first_kind, .. } => {
                if let AttributeContents::OutputPin { kind: ref second_kind, .. } = second_attribute.contents {
                    // note: rust automatically derefs when doing comparisons!
                    if first_kind == second_kind {
                        Some((first_attribute.id, second_attribute.id))
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
            AttributeContents::OutputPin { kind: first_kind, .. } => {
                if let AttributeContents::InputPin { kind: ref second_kind, .. } = second_attribute.contents {
                    // note: rust automatically derefs when doing comparisons!
                    if first_kind == second_kind {
                        Some((second_attribute.id, first_attribute.id))
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
            _ => None
        }

    }

    // checks if the two pins are compatible and if they are, return a sorted pair:
    // the first id is the one belonging to the input attribute.
    fn check_link_creation(attributes: &BTreeMap::<AttributeID, Attribute>, first_id: AttributeID, second_id: AttributeID) -> Option<(AttributeID, AttributeID)> {
        let first_attribute_opt = attributes.get(&first_id);
        let second_attribute_opt = attributes.get(&second_id);
        match (first_attribute_opt, second_attribute_opt) {
            // if both attributes actually exist, check if they are compatible
            (Some(first_attribute), Some(second_attribute)) => {
                Self::check_pin_compatibility(first_attribute, second_attribute)
            },
            // TODO: maybe log a warning instead of panic?
            (None, Some(_)) => unreachable!("When attempting to create a link, the first attribute was not found in the map"),
            (Some(_), None) => unreachable!("When attempting to create a link, the second attribute was not found in the map"),
            (None, None) => unreachable!("When attempting to create a link, none of the two attributes was found in the map"),
        }
    }

    pub fn add_interval_node(&mut self) {
        let node_id = self.get_next_id();
        let variable = Attribute {
            id: self.get_next_id(),
            node_id,
            contents: AttributeContents::Text {
                label: ImString::new(" name"),
                string: ImString::new(""),
            }
        };
        let begin = Attribute {
            id: self.get_next_id(),
            node_id,
            contents: AttributeContents::Text {
                label: ImString::new("begin"),
                string: ImString::new(""),
            }
        };
        let end = Attribute {
            id: self.get_next_id(),
            node_id,
            contents: AttributeContents::Text {
                label: ImString::new("  end"),
                string: ImString::new(""),
            }
        };
        let output = Attribute {
            id: self.get_next_id(),
            node_id,
            contents: AttributeContents::OutputPin {
                label: ImString::new("interval"),
                kind: DataKind::Interval,
            }
        };
        let node = Node {
            id: node_id,
            title: ImString::new("Interval node"),
            error: None,
            contents: NodeContents::Interval {
                variable: variable.id,
                begin: begin.id,
                end: end.id,
                output: output.id,
            }
        };
        self.nodes.insert(node_id, node);
        self.attributes.insert(variable.id, variable);
        self.attributes.insert(begin.id, begin);
        self.attributes.insert(end.id, end);
        self.attributes.insert(output.id, output);
    }

    pub fn add_curve_node(&mut self) {
        let node_id = self.get_next_id();
        let fx = Attribute {
            id: self.get_next_id(),
            node_id,
            contents: AttributeContents::Text {
                label: ImString::new("fx"),
                string: ImString::new(""),
            }
        };
        let fy = Attribute {
            id: self.get_next_id(),
            node_id,
            contents: AttributeContents::Text {
                label: ImString::new("fy"),
                string: ImString::new(""),
            }
        };
        let fz = Attribute {
            id: self.get_next_id(),
            node_id,
            contents: AttributeContents::Text {
                label: ImString::new("fz"),
                string: ImString::new(""),
            }
        };
        let interval = Attribute {
            id: self.get_next_id(),
            node_id,
            contents: AttributeContents::InputPin {
                label: ImString::new("interval"),
                kind: DataKind::Interval,
            }
        };
        let output = Attribute {
            id: self.get_next_id(),
            node_id,
            contents: AttributeContents::OutputPin {
                label: ImString::new("geometry"),
                kind: DataKind::Geometry,
            }
        };
        let node = Node {
            id: node_id,
            title: ImString::new("Curve node"),
            error: None,
            contents: NodeContents::Curve {
                fx: fx.id,
                fy: fy.id,
                fz: fz.id,
                interval: interval.id,
                output: output.id,
            }
        };
        self.nodes.insert(node_id, node);
        self.attributes.insert(fx.id, fx);
        self.attributes.insert(fy.id, fy);
        self.attributes.insert(fz.id, fz);
        self.attributes.insert(interval.id, interval);
        self.attributes.insert(output.id, output);
    }

    pub fn add_rendering_node(&mut self) {
        let node_id = self.get_next_id();
        let geometry = Attribute {
            id: self.get_next_id(),
            node_id,
            contents: AttributeContents::InputPin {
                label: ImString::new("geometry"),
                kind: DataKind::Geometry,
            }
        };
        let node = Node {
            id: node_id,
            title: ImString::new("Curve node"),
            error: None,
            contents: NodeContents::Rendering {
                geometry: geometry.id,
            }
        };
        self.nodes.insert(node_id, node);
        self.attributes.insert(geometry.id, geometry);
    }

    pub fn get_next_id(&mut self) -> i32 {
        let temp = self.next_id;
        self.next_id += 1;
        temp
    }
}
