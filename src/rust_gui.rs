use crate::compute_graph::globals::Globals;
use crate::file_io;
use crate::state::{Action, State};
pub type BlockId = i32;
pub type PrefabId = i32;
pub type TextureId = u32;
pub type FontId = u32;

const MAX_UNDO_HISTORY : usize = 10;
pub type MaskIds = [TextureId; 5];
pub type MaterialIds = Vec<TextureId>;

pub struct Availables {
    pub mask_ids: MaskIds,
    pub material_ids: MaterialIds,
    pub model_names: Vec<String>,
}

pub struct Gui {
    pub scene_texture_id: TextureId,
    pub new_variable_buffer: String,
    pub new_variable_error: Option<String>,
    graph_fonts: Vec<u32>,
    winit_proxy: winit::event_loop::EventLoopProxy<super::CustomEvent>,
    undo_stack: std::collections::VecDeque<(f64, String)>,
    undo_cursor: usize,
    pub graph_edited: bool,
    pub added_zoom: f32,
    accumulated_zoom: f32,
    selected_object: Option<BlockId>,
    availables: Availables,
    axes_length: i32,
    axes_marks_size: f32,
    labels_size: f32,
    pub opened_tab: [bool; 3],
}

#[derive(Debug)]
pub struct SceneRectangle {
    pub position: [f32; 2],
    pub size: [f32; 2],
}

impl Gui {
    pub fn new(winit_proxy: winit::event_loop::EventLoopProxy<super::CustomEvent>, scene_texture_id: TextureId, availables: Availables, graph_fonts: Vec<FontId>) -> Self {
        // when we initialize a GUI, we want to set the first undo_stack element to a completely empty graph
        use super::node_graph::NodeGraph;
        let empty_graph = NodeGraph::default();
        Self {
            accumulated_zoom: 0.0,
            added_zoom: 0.0,
            availables,
            scene_texture_id,
            graph_fonts,
            winit_proxy,
            undo_stack: vec![(0.0, serde_json::to_string(&empty_graph).unwrap())].into(),
            undo_cursor: 0,
            new_variable_buffer: String::with_capacity(8),
            new_variable_error: None,
            graph_edited: false,
            selected_object: None,
            axes_length: 2,
            axes_marks_size: 0.075,
            labels_size: 0.15,
            opened_tab: [true, false, false],
        }
    }

    /// this function clears up the user undo history, setting the current state as the
    /// only existing one on the undo stack.
    /// an action that is required when creating a new file or opening an existing one
    pub fn reset_undo_history(&mut self, state: &State) {
        self.undo_stack = vec![(0.0, serde_json::to_string(&state.user.node_graph).unwrap())].into();
        self.undo_cursor = 0;
        self.graph_edited = false;
    }

    pub fn reset_nongraph_data(&mut self) {
        self.selected_object = None;
        self.new_variable_buffer.clear();
        self.new_variable_error = None;
    }

    pub fn issue_undo(&mut self, state: &mut State, timestamp: f64) {
        // if the user is actively editing a node, we want to stop the editing and issue a savestate!
        if state.user.node_graph.currently_editing() {
            // stop the editing on the imgui side
            //use crate::cpp_gui::ImGui;
            //ImGui::ClearActiveID(); // TODO GUI
            // stop the editing on the rust side
            state.user.node_graph.stop_editing();
            // issue a savestate
            self.issue_savestate(state, timestamp);
        }
        if self.undo_cursor != 0 {
            let zoom_level = state.user.node_graph.zoom_level;
            self.undo_cursor -= 1;
            let old_state = self.undo_stack.get(self.undo_cursor).unwrap();
            // println!("Restored state from {} seconds ago", ui.time() - old_state.0);
            state.user.node_graph = serde_json::from_str(&old_state.1).unwrap();
            state.user.node_graph.zoom_level = zoom_level;
            state.user.node_graph.push_positions_to_imnodes();
        }
    }

    pub fn issue_savestate(&mut self, state: &mut State, timestamp: f64) {
        if self.undo_stack.len() == MAX_UNDO_HISTORY {
            // If the length is already equal to MAX_UNDO_HISTORY, pop the first element
            // there is no need to manipulate the undo_cursor because the element that we will
            // insert will become the one that the undo_cursor already points at.
            self.undo_stack.pop_front();
        } else {
            // Otherwise truncate the stack so that the "Redo" history is not accessible anymore.
            // The truncate() function takes the number of elements that we want to preserve.
            // After truncate move the cursor forward, so that it will point to the element
            // that we are about to add.
            let preserved_elements = self.undo_cursor + 1;
            self.undo_stack.truncate(preserved_elements);
            self.undo_cursor += 1;
        }
        let serialized_graph = serde_json::to_string(&state.user.node_graph).unwrap();
        self.undo_stack.push_back((timestamp, serialized_graph));
        self.graph_edited = true;
    }

    pub fn issue_redo(&mut self, state: &mut State) {
        if self.undo_cursor != self.undo_stack.len()-1 {
            let zoom_level = state.user.node_graph.zoom_level;
            self.undo_cursor += 1;
            let restored_state = self.undo_stack.get(self.undo_cursor).unwrap();
            // println!("Restored state from {} seconds ago", ui.time() - restored_state.0);
            state.user.node_graph = serde_json::from_str(&restored_state.1).unwrap();
            state.user.node_graph.zoom_level = zoom_level;
            state.user.node_graph.push_positions_to_imnodes();
            self.graph_edited = true;
        }
    }
}
