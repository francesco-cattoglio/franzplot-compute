use std::collections::BTreeMap;
use crate::cpp_gui::imnodes;
use crate::cpp_gui::PinShape;
use imgui::*;

pub type AttributeID = i32;
pub type NodeID = i32;

pub struct Node {
    id: i32,
    title: ImString,
    contents: NodeContents,
}

pub enum NodeContents {
    Interval {
        variable: AttributeID,
        begin: AttributeID,
        end: AttributeID,
        out: AttributeID,
    },
    Curve {
        interval: AttributeID,
        fx: AttributeID,
        fy: AttributeID,
        fz: AttributeID,
        out: AttributeID,
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
    pub fn render(&mut self, ui: &imgui::Ui<'_>, attributes: &mut BTreeMap<AttributeID, Attribute>) {
        imnodes::BeginNode(self.id);
            imnodes::BeginNodeTitleBar();
            ui.text("works ⚠");
            imnodes::EndNodeTitleBar();
            match &self.contents {
                NodeContents::Interval {
                    variable, begin, end, out,
                } => {
                    attributes.get_mut(out).unwrap().render(ui);
                    attributes.get_mut(variable).unwrap().render(ui);
                    attributes.get_mut(begin).unwrap().render(ui);
                    attributes.get_mut(end).unwrap().render(ui);
                },
                NodeContents::Curve {
                    interval, fx, fy, fz, out,
                } => {
                    attributes.get_mut(out).unwrap().render(ui);
                    attributes.get_mut(interval).unwrap().render(ui);
                    attributes.get_mut(fx).unwrap().render(ui);
                    attributes.get_mut(fy).unwrap().render(ui);
                    attributes.get_mut(fz).unwrap().render(ui);
                }
                _ => {}
            }
        imnodes::EndNode();
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
        pin_shape: PinShape,
    },
    OutputPin {
        label: ImString,
        pin_shape: PinShape,
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
                label, pin_shape,
            } => {
                imnodes::BeginInputAttribute(self.id, *pin_shape);
                ui.text(label);
                imnodes::EndInputAttribute();
            },
            AttributeContents::OutputPin {
                label, pin_shape,
            } => {
                imnodes::BeginOutputAttribute(self.id, *pin_shape);
                ui.text(label);
                imnodes::EndOutputAttribute();
            },
            AttributeContents::Text{
                label, string,
            } => {
                imnodes::BeginStaticAttribute(self.id);
                ui.text(&label);
                ui.same_line(0.0);
                InputText::new(ui, im_str!(""), string).build();
                imnodes::EndStaticAttribute();
            },
            _ => {
            }
        }
    }
}

// TODO: get a constructor and make next_id private!
pub struct NodeGraph {
    pub nodes: BTreeMap::<NodeID, Node>,
    pub attributes: BTreeMap::<AttributeID, Attribute>,
    pub next_id: i32,
}

impl NodeGraph {
    pub fn render(&mut self, ui: &imgui::Ui<'_>) {
        imnodes::BeginNodeEditor();

        for node in self.nodes.values_mut() {
            node.render(ui, &mut self.attributes);
        }
        imnodes::EndNodeEditor();
    }

    pub fn add_interval_node(&mut self) {
        let node_id = self.get_next_id();
        let variable = Attribute {
            id: self.get_next_id(),
            node_id,
            contents: AttributeContents::Text {
                label: ImString::new("name"),
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
        let out = Attribute {
            id: self.get_next_id(),
            node_id,
            contents: AttributeContents::OutputPin {
                label: ImString::new("out"),
                pin_shape: PinShape::Quad,
            }
        };
        let node = Node {
            title: ImString::new("Interval node"),
            id: node_id,
            contents: NodeContents::Interval {
                variable: variable.id,
                begin: begin.id,
                end: end.id,
                out: out.id,
            }
        };
        self.nodes.insert(node_id, node);
        self.attributes.insert(variable.id, variable);
        self.attributes.insert(begin.id, begin);
        self.attributes.insert(end.id, end);
        self.attributes.insert(out.id, out);
    }

    pub fn get_next_id(&mut self) -> i32 {
        let temp = self.next_id;
        self.next_id += 1;
        temp
    }
}
