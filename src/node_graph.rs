use std::collections::BTreeMap;
use imgui::*;

pub type AttributeID = i32;
pub type NodeID = i32;

pub struct Node {
    title: String,
    contents: NodeContents,
}

pub enum NodeContents {
    Interval {
        variable: AttributeID,
        begin: AttributeID,
        end: AttributeID,
    },
    Curve {
        interval: AttributeID,
        fx: AttributeID,
        fy: AttributeID,
        fz: AttributeID,
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
    pub fn render_title(&self) {
    }
}

pub struct Attribute {
    id: AttributeID,
    node_id: NodeID,
    contents: AttributeContents,
}

pub enum AttributeContents {
    InputPin {
        label: &'static str,
    },
    OutputPin {
        label: &'static str,
    },
    Text {
        label: &'static str,
        string: String,
    },
    Unknown {
        label: &'static str,
    }
}

impl Attribute {
    pub fn render(&self) {
        match self.contents {
            AttributeContents::InputPin{..} => {

            },
            AttributeContents::Text{..} => {
            },
            _ => {
            }
        }
    }
}

pub struct NodeGraph {
    pub nodes: BTreeMap::<NodeID, Node>,
    pub attributes: BTreeMap::<AttributeID, Attribute>,
}

impl NodeGraph {
    pub fn render(&mut self, ui: &imgui::Ui<'_>) {
        use crate::cpp_gui::ffi2::*;
        BeginNodeEditor();
        BeginNode(12);
            BeginNodeTitleBar();
            ui.text("works ⚠");
            EndNodeTitleBar();
            BeginInputAttribute(2, crate::cpp_gui::PinShape::Circle);
            ui.text("works 2 ⚠");
            EndInputAttribute();
        EndNode();

        BeginNode(14);
            BeginNodeTitleBar();
            ui.text("works  ⚠");
            EndNodeTitleBar();
            BeginOutputAttribute(3, crate::cpp_gui::PinShape::CircleFilled);
            ui.text("works 3 ⚠");
            EndOutputAttribute();
        EndNode();

        EndNodeEditor();
    }
}
