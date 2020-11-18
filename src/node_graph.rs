use std::collections::BTreeMap;
use imgui::*;

pub type AttributeID = i32;
pub type NodeID = i32;

pub struct Node {
    node_title: String,
    node_contents: NodeContents,
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

pub enum Attribute {
    Pin {
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
        match self {
            Attribute::Pin{..} => {
            },
            Attribute::Text{..} => {
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
        let window = imgui::Window::new(im_str!("testt"));
        if let Some(token) = window.begin(ui) {
            crate::cpp_gui::ffi::BeginNodeEditor();
            crate::cpp_gui::ffi::BeginNode(12);
            crate::cpp_gui::ffi::BeginNodeTitleBar();
            ui.text("works ⚠");
            crate::cpp_gui::ffi::EndNodeTitleBar();
            crate::cpp_gui::ffi::EndNode();
            crate::cpp_gui::ffi::EndNodeEditor();
            token.end(ui);
        }
    }
}
