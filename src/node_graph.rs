use std::collections::HashMap;
use crate::cpp_gui::imnodes;
use crate::cpp_gui::PinShape;
use serde::{Serialize, Deserialize};
use imgui::*;

pub type AttributeID = i32;
pub type NodeID = i32;

#[derive(Clone, PartialEq, Deserialize, Serialize)]
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

#[derive(Deserialize, Serialize)]
pub struct Attribute {
    node_id: NodeID,
    contents: AttributeContents,
}

#[derive(Clone, Deserialize, Serialize)]
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
    QualitySlider {
        label: String,
        quality: i32,
    },
    MatrixRow {
        col_1: String,
        col_2: String,
        col_3: String,
        col_4: String,
    },
    Unknown {
        label: String,
    }
}

impl Attribute {
    pub fn render(&mut self, ui: &imgui::Ui<'_>, id: AttributeID) {
        match &mut self.contents {
            AttributeContents::InputPin {
                label, kind,
            } => {
                imnodes::BeginInputAttribute(id, kind.to_pin_shape());
                ui.text(label);
                imnodes::EndInputAttribute();
            },
            AttributeContents::OutputPin {
                label, kind,
            } => {
                imnodes::BeginOutputAttribute(id, kind.to_pin_shape());
                ui.text(label);
                imnodes::EndOutputAttribute();
            },
            AttributeContents::Text{
                label, string,
            } => {
                imnodes::BeginStaticAttribute(id);
                ui.text(&label);
                ui.same_line(0.0);
                ui.set_next_item_width(80.0);
                let mut imstring = ImString::new(string.clone());
                InputText::new(ui, im_str!(""), &mut imstring)
                    .resize_buffer(true)
                    .build();
                *string = imstring.to_string();
                imnodes::EndStaticAttribute();
            },
            AttributeContents::QualitySlider {
                label, quality,
            } => {
                imnodes::BeginStaticAttribute(id);
                ui.text(&label);
                ui.same_line(0.0);
                ui.set_next_item_width(55.0);
                Slider::new(im_str!(""))
                    .range(4 ..= 16)
                    .build(ui, quality);
                imnodes::EndStaticAttribute();
            },
            AttributeContents::MatrixRow {
                col_1, col_2, col_3, col_4,
            } => {
                imnodes::BeginStaticAttribute(id);

                let width_token = ui.push_item_width(50.0);
                let mut imstring: ImString;

                // TODO: this is kinda ugly
                imstring = ImString::new(col_1.clone());
                InputText::new(ui, im_str!("##1"), &mut imstring)
                    .resize_buffer(true)
                    .build();
                *col_1 = imstring.to_string();

                ui.same_line(0.0);

                imstring = ImString::new(col_2.clone());
                InputText::new(ui, im_str!("##2"), &mut imstring)
                    .resize_buffer(true)
                    .build();
                *col_2 = imstring.to_string();

                ui.same_line(0.0);

                imstring = ImString::new(col_3.clone());
                InputText::new(ui, im_str!("##3"), &mut imstring)
                    .resize_buffer(true)
                    .build();
                *col_3 = imstring.to_string();

                ui.same_line(0.0);

                imstring = ImString::new(col_4.clone());
                InputText::new(ui, im_str!("##4"), &mut imstring)
                    .resize_buffer(true)
                    .build();
                *col_4 = imstring.to_string();

                imnodes::EndStaticAttribute();
                width_token.pop(ui);
            },
            AttributeContents::Unknown {
                ..
            } => {
                unimplemented!()
            }
        }
    }

    pub fn render_list(ui: &imgui::Ui<'_>, attributes: &mut Vec<Option<Attribute>>, attribute_id_list: Vec<AttributeID>) {
        for id in attribute_id_list.into_iter() {
            if let Some(Some(attribute)) = attributes.get_mut(id as usize) {
                attribute.render(ui, id);
            }
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Node {
    title: String,
    error: Option<GraphError>,
    contents: NodeContents,
}

#[derive(Clone, Deserialize, Serialize)]
pub enum NodeContents {
    Interval {
        variable: AttributeID,
        begin: AttributeID,
        end: AttributeID,
        quality: AttributeID,
        output: AttributeID,
    },
    Point {
        x: AttributeID,
        y: AttributeID,
        z: AttributeID,
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
    Rendering {
        geometry: AttributeID,
    },
    Group
}

impl Node {
    pub fn contents(&self) -> &NodeContents {
        &self.contents
    }

    pub fn render(&mut self, ui: &imgui::Ui<'_>, attributes: &mut Vec<Option<Attribute>>) {
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
            // TODO: macro? NodeContents-to-render-list function?
            // we cannot reuse the get_owned_nodes because there will be a Group kind
            // of node in the future...
            match self.contents {
                NodeContents::Interval {
                    variable, begin, end, quality, output,
                } => {
                    Attribute::render_list(ui, attributes, vec![variable, begin, end, quality, output,]);
                },
                NodeContents::Point {
                    x, y, z, output,
                } => {
                    Attribute::render_list(ui, attributes, vec![x, y, z, output,]);
                },
                NodeContents::Curve {
                    interval, fx, fy, fz, output,
                } => {
                    Attribute::render_list(ui, attributes, vec![interval, fx, fy, fz, output,]);
                },
                NodeContents::Surface {
                    interval_1, interval_2, fx, fy, fz, output,
                } => {
                    Attribute::render_list(ui, attributes, vec![interval_1, interval_2, fx, fy, fz, output,]);
                },
                NodeContents::Transform {
                    geometry, matrix, output,
                } => {
                    Attribute::render_list(ui, attributes, vec![geometry, matrix, output,]);
                },
                NodeContents::Matrix {
                    interval, row_1, row_2, row_3, output,
                } => {
                    Attribute::render_list(ui, attributes, vec![interval, row_1, row_2, row_3, output,]);
                },
                NodeContents::Rendering {
                    geometry,
                } => {
                    Attribute::render_list(ui, attributes, vec![geometry,]);
                },
                NodeContents::Group => { unimplemented!() }
            }
    }

    pub fn get_input_nodes(&self, graph: &NodeGraph) -> Vec::<Option<NodeID>> {
        match self.contents {
            NodeContents::Interval { .. } => {
                vec![]
            },
            NodeContents::Point { .. } => {
                vec![]
            },
            NodeContents::Curve { interval, .. } => {
                vec![graph.get_attribute_as_linked_node(interval)]
            },
            NodeContents::Surface { interval_1, interval_2, .. } => {
                vec![
                    graph.get_attribute_as_linked_node(interval_1),
                    graph.get_attribute_as_linked_node(interval_2),
                ]
            },
            NodeContents::Transform { geometry, matrix, .. } => {
                vec![
                    graph.get_attribute_as_linked_node(geometry),
                    graph.get_attribute_as_linked_node(matrix),
                ]
            },
            NodeContents::Matrix { interval, .. } => {
                vec![
                    graph.get_attribute_as_linked_node(interval),
                ]
            },
            NodeContents::Rendering { geometry, } => {
                vec![
                    graph.get_attribute_as_linked_node(geometry)
                ]
            },
            NodeContents::Group => {
                unimplemented!()
            }
        }
    }

    // TODO: macro? NodeContents-to-attributes-list function?
    pub fn get_owned_attributes(&mut self) -> Vec::<&mut AttributeID> {
        match &mut self.contents {
            NodeContents::Interval {
                variable, begin, end, quality, output,
            } => {
                vec![variable, begin, end, quality, output]
            },
            NodeContents::Point {
                x, y, z, output
            } => {
                vec![x, y, z, output]
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
            NodeContents::Rendering {
                geometry,
            } => {
                vec![geometry]
            },
            NodeContents::Group => {
                unimplemented!()
            }
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub enum Severity {
    Warning,
    Error
}
#[derive(Clone, Deserialize, Serialize)]
pub struct GraphError {
    pub node_id: NodeID,
    pub severity: Severity,
    pub message: String,
}

#[derive(Deserialize, Serialize)]
pub struct NodeGraph {
    nodes: Vec<Option<Node>>,
    attributes: Vec<Option<Attribute>>,
    links: HashMap::<AttributeID, AttributeID>,
    free_nodes_list: Vec<NodeID>,
    free_attributes_list: Vec<AttributeID>,
    #[serde(skip)]
    last_hovered_node: NodeID,
    #[serde(skip)]
    last_hovered_link: AttributeID,
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
            last_hovered_node: -1,
            last_hovered_link: -1,
        }
    }
}

impl NodeGraph {
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

    pub fn clone_node(&self, node_id: NodeID) -> Option<(Node, Vec<AttributeContents>)> {
        // to clone a node, we need to clone its kind, clone its attributes
        // and then remap all its attributes to the newly-created "local attributes array"
        let mut cloned_node = self.get_node(node_id)?.clone();
        let owned_attributes = cloned_node.get_owned_attributes();

        let mut cloned_attributes = Vec::<AttributeContents>::with_capacity(owned_attributes.len());
        // for each (index, reference to the attribute_id contained in our node)
        for (i, attribute_id) in owned_attributes.into_iter().enumerate() {
            // clone the attribute originally pointed at by attribute_id
            let attribute = self.attributes.get(*attribute_id as usize).unwrap().as_ref().unwrap();
            cloned_attributes.push(attribute.contents.clone());
            // and then modify the attribute_id inside the node with the current index
            *attribute_id = i as AttributeID;
        }
        Some((cloned_node, cloned_attributes))
    }

    pub fn insert_node(&mut self, mut node: Node, attributes_contents: Vec<AttributeContents>) -> NodeID {
        // make a check: the list of owned attributes must have the same
        // length as the attributes vector
        let owned_attributes = node.get_owned_attributes();
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
        // into iter returns a reference to each attribute stored in our node.\
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
        for (idx, maybe_node) in self.nodes.iter_mut().enumerate() {
            if let Some(node) = maybe_node.as_mut() {
                imnodes::BeginNode(idx as NodeID);
                node.render(ui, &mut self.attributes);
                imnodes::EndNode();
            }
        }

        // we need to end the node editor before doing any interaction
        // (e.g: right clicks, node creation)
        imnodes::EndNodeEditor();

        // Process right click
        let mouse_delta = ui.mouse_drag_delta_with_threshold(MouseButton::Right, 4.0);
        let right_click_popup: bool = ui.is_window_focused_with_flags(WindowFocusedFlags::ROOT_AND_CHILD_WINDOWS)
            && !ui.is_any_item_hovered()
            && ui.is_mouse_released(MouseButton::Right)
            && mouse_delta == [0.0, 0.0]; // exact comparison is fine due to GetMouseDragDelta threshold

        let mut hovered_id: i32 = -1;
        if right_click_popup {
            println!("rcd");
            if imnodes::IsNodeHovered(&mut hovered_id) {
                self.last_hovered_node = hovered_id;
                ui.open_popup(im_str!("Node menu"));
            } else if imnodes::IsLinkHovered(&mut hovered_id) {
                self.last_hovered_link = hovered_id;
                ui.open_popup(im_str!("Link menu"));
            } else {
                ui.open_popup(im_str!("Add menu"));
            }
        }

        ui.popup(im_str!("Node menu"), || {
            let clicked_pos = ui.mouse_pos_on_opening_current_popup();
            if MenuItem::new(im_str!("delete node")).build(ui) {
                println!("need to remove {}", self.last_hovered_node);
                self.remove_node(self.last_hovered_node);
            }
            if MenuItem::new(im_str!("duplicate node")).build(ui) {
                let (cloned_node, cloned_attributes) = self.clone_node(self.last_hovered_node).unwrap();
                let new_node_id = self.insert_node(cloned_node, cloned_attributes);
                imnodes::SetNodeScreenSpacePos(new_node_id, clicked_pos[0]+20.0, clicked_pos[1]+20.0);
            }
            if MenuItem::new(im_str!("rename node")).build(ui) {
                println!("need to rename {}", self.last_hovered_node);
            }
        });

        ui.popup(im_str!("Link menu"), || {
            if MenuItem::new(im_str!("delete link")).build(ui) {
                println!("need to remove {}", self.last_hovered_link);
                self.links.remove(&self.last_hovered_link);
            }
        });

        let style_token = ui.push_style_var(StyleVar::WindowPadding([8.0, 8.0]));
        ui.popup(im_str!("Add menu"), || {
            let clicked_pos = ui.mouse_pos_on_opening_current_popup();
            if MenuItem::new(im_str!("Interval")).build(ui) {
                let new_node_id = self.add_interval_node();
                imnodes::SetNodeScreenSpacePos(new_node_id, clicked_pos[0], clicked_pos[1]);
            }

            ui.menu(im_str!("Geometries"), true, || {
                if MenuItem::new(im_str!("Curve")).build(ui) {
                    let new_node_id = self.add_curve_node();
                    imnodes::SetNodeScreenSpacePos(new_node_id, clicked_pos[0], clicked_pos[1]);
                }
                if MenuItem::new(im_str!("Surface")).build(ui) {
                    let new_node_id = self.add_surface_node();
                    imnodes::SetNodeScreenSpacePos(new_node_id, clicked_pos[0], clicked_pos[1]);
                }
            }); // Geometries menu ends here

            ui.menu(im_str!("Transformations"), true, || {
                if MenuItem::new(im_str!("Matrix")).build(ui) {
                    let new_node_id = self.add_matrix_node();
                    imnodes::SetNodeScreenSpacePos(new_node_id, clicked_pos[0], clicked_pos[1]);
                }
                if MenuItem::new(im_str!("Transform")).build(ui) {
                    let new_node_id = self.add_transform_node();
                    imnodes::SetNodeScreenSpacePos(new_node_id, clicked_pos[0], clicked_pos[1]);
                }
            }); // Transformations menu ends here

            if MenuItem::new(im_str!("Point")).build(ui) {
                let new_node_id = self.add_point_node();
                imnodes::SetNodeScreenSpacePos(new_node_id, clicked_pos[0], clicked_pos[1]);
            }
            if MenuItem::new(im_str!("Rendering")).build(ui) {
                let new_node_id = self.add_rendering_node();
                imnodes::SetNodeScreenSpacePos(new_node_id, clicked_pos[0], clicked_pos[1]);
            }
        }); // "Add" closure ends here
        style_token.pop(ui);

        // check if a link was created
        let mut start_attribute_id: AttributeID = -1;
        let mut end_attribute_id: AttributeID = -1;
        if imnodes::IsLinkCreated(&mut start_attribute_id, &mut end_attribute_id) {
            let maybe_link = Self::check_link_creation(&self.attributes, start_attribute_id, end_attribute_id);
            // check which one of the two attributes is the input attribute and which is the output
            if let Some((input_id, output_id)) = maybe_link {
                self.links.insert(input_id, output_id);
            }
        }
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

    pub fn get_node(&self, id: NodeID) -> Option<&Node> {
        if let Some(maybe_node) = self.nodes.get(id as usize) {
            maybe_node.as_ref()
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

    fn remove_node(&mut self, node_id: NodeID) {
        // try to remove this node_id from the map
        if let Some(slot) = self.nodes.get_mut(node_id as usize) {
            // slot is a reference to the option! By taking() it, we effectively
            // remove the old node from the graph's nodes
            let mut old_node = slot.take().unwrap(); // this unwrap asserts that the old node is Some(thing)
            self.free_nodes_list.push(node_id);
            // if the node exists, get a list of all the attributes belonging to it
            // remove all the attributes of the attributes map.
            let list_of_attributes: Vec<AttributeID> = old_node
                    .get_owned_attributes()
                    .into_iter()
                    .map(|x| *x)
                    .collect();

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

    pub fn get_attribute_as_string(&self, attribute_id: AttributeID) -> Option<String> {
        // first, we need to check if the attribute_id actually exists in our attributes map
        if let Some(Some(attribute)) = self.attributes.get(attribute_id as usize) {
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

    // TODO: this is kinda bad as well. A rework on matrix attributes might be needed.
    pub fn get_attribute_as_multistring(&self, attribute_id: AttributeID) -> Vec<String> {
        // first, we need to check if the attribute_id actually exists in our attributes map
        if let Some(Some(attribute)) = self.attributes.get(attribute_id as usize) {
            // if it exists, then we need to check if it is an input pin
            if let AttributeContents::MatrixRow{ col_1, col_2, col_3, col_4, } = &attribute.contents {
                vec![
                    col_1.to_string(),
                    col_2.to_string(),
                    col_3.to_string(),
                    col_4.to_string(),
                ]
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    pub fn get_attribute_as_linked_node(&self, input_attribute_id: AttributeID) -> Option<NodeID> {
        // first, we need to check if the input_attribute_id actually exists in our attributes map
        if let Some(Some(attribute)) = self.attributes.get(input_attribute_id as usize) {
            // if it exists, then we need to check if it is an input pin
            if let AttributeContents::InputPin{..} = &attribute.contents {
                // and then we can finally check if it is actually linked to something!
                if let Some(output_attribute_id) = self.links.get(&input_attribute_id) {
                    // we can assert that if there is a link, then the linked one exists
                    let linked_node_id = self.attributes[*output_attribute_id as usize].as_ref().unwrap().node_id;
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

    pub fn add_interval_node(&mut self) -> NodeID {
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
            AttributeContents::QualitySlider {
                label: String::from("quality"),
                quality: 4,
            },
            AttributeContents::OutputPin {
                label: String::from("interval"),
                kind: DataKind::Interval,
            }
        ];
        let node = Node {
            title: String::from("Interval"),
            error: None,
            contents: NodeContents::Interval {
                variable: 0,
                begin: 1,
                end: 2,
                quality: 3,
                output: 4,
            }
        };
        self.insert_node(node, attributes_contents)
    }

    pub fn add_point_node(&mut self) -> NodeID {
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
        let node = Node {
            title: String::from("Point"),
            error: None,
            contents: NodeContents::Point {
                x: 0,
                y: 1,
                z: 2,
                output: 3,
            }
        };
        self.insert_node(node, attributes_contents)
    }

    pub fn add_curve_node(&mut self) -> NodeID {
        let attributes_contents = vec![
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
            AttributeContents::InputPin {
                label: String::from("interval"),
                kind: DataKind::Interval,
            },
            AttributeContents::OutputPin {
                label: String::from("geometry"),
                kind: DataKind::Geometry,
            }
        ];
        let node = Node {
            title: String::from("Curve"),
            error: None,
            contents: NodeContents::Curve {
                fx: 0,
                fy: 1,
                fz: 2,
                interval: 3,
                output: 4,
            }
        };
        self.insert_node(node, attributes_contents)
    }

    pub fn add_surface_node(&mut self) -> NodeID {
        let attributes_contents = vec![
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
            AttributeContents::InputPin {
                label: String::from("interval 1"),
                kind: DataKind::Interval,
            },
            AttributeContents::InputPin {
                label: String::from("interval 2"),
                kind: DataKind::Interval,
            },
            AttributeContents::OutputPin {
                label: String::from("geometry"),
                kind: DataKind::Geometry,
            }
        ];
        let node = Node {
            title: String::from("Surface"),
            error: None,
            contents: NodeContents::Surface {
                fx: 0,
                fy: 1,
                fz: 2,
                interval_1: 3,
                interval_2: 4,
                output: 5,
            }
        };
        self.insert_node(node, attributes_contents)
    }

    pub fn add_rendering_node(&mut self) -> NodeID {
        let attributes_contents = vec![
            AttributeContents::InputPin {
                label: String::from("geometry"),
                kind: DataKind::Geometry,
            }
        ];
        let node = Node {
            title: String::from("Curve"),
            error: None,
            contents: NodeContents::Rendering {
                geometry: 0,
            }
        };
        self.insert_node(node, attributes_contents)
    }

    pub fn add_transform_node(&mut self) -> NodeID {
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
        let node = Node {
            title: String::from("Transform"),
            error: None,
            contents: NodeContents::Transform {
                geometry: 0,
                matrix: 1,
                output: 2,
            }
        };
        self.insert_node(node, attributes_contents)
    }

    pub fn add_matrix_node(&mut self) -> NodeID {
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
        let node = Node {
            title: String::from("Matrix"),
            error: None,
            contents: NodeContents::Matrix {
                interval: 0,
                row_1: 1,
                row_2: 2,
                row_3: 3,
                output: 4,
            }
        };
        self.insert_node(node, attributes_contents)
    }

}
