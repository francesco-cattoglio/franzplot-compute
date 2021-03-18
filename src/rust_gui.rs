use imgui::*;
use crate::file_io;
use crate::state::State;
use crate::computable_scene::compute_block::BlockId;

const MAX_UNDO_HISTORY : usize = 10;
pub type MaskIds = [TextureId; 5];
pub type MaterialIds = Vec<TextureId>;

pub struct Availables {
    pub mask_ids: MaskIds,
    pub material_ids: MaterialIds,
    pub model_names: Vec<ImString>,
}

pub struct Gui {
    pub scene_texture_id: TextureId,
    pub new_global_buffer: ImString,
    graph_fonts: Vec<imgui::FontId>,
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
            new_global_buffer: ImString::with_capacity(8),
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
        self.undo_stack = vec![(0.0, serde_json::to_string(&state.user.graph).unwrap())].into();
        self.undo_cursor = 0;
        self.graph_edited = false;
    }

    pub fn reset_nongraph_data(&mut self) {
        self.selected_object = None;
        self.new_global_buffer.clear();
    }

    pub fn issue_undo(&mut self, state: &mut State, timestamp: f64) {
        // if the user is actively editing a node, we want to stop the editing and issue a savestate!
        if state.user.graph.currently_editing() {
            // stop the editing on the imgui side
            use crate::cpp_gui::ImGui;
            ImGui::ClearActiveID();
            // stop the editing on the rust side
            state.user.graph.stop_editing();
            // issue a savestate
            self.issue_savestate(state, timestamp);
        }
        if self.undo_cursor != 0 {
            let zoom_level = state.user.graph.zoom_level;
            self.undo_cursor -= 1;
            let old_state = self.undo_stack.get(self.undo_cursor).unwrap();
            // println!("Restored state from {} seconds ago", ui.time() - old_state.0);
            state.user.graph = serde_json::from_str(&old_state.1).unwrap();
            state.user.graph.zoom_level = zoom_level;
            state.user.graph.push_positions_to_imnodes();
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
        let serialized_graph = serde_json::to_string(&state.user.graph).unwrap();
        self.undo_stack.push_back((timestamp, serialized_graph));
        self.graph_edited = true;
    }

    pub fn issue_redo(&mut self, state: &mut State) {
        if self.undo_cursor != self.undo_stack.len()-1 {
            let zoom_level = state.user.graph.zoom_level;
            self.undo_cursor += 1;
            let restored_state = self.undo_stack.get(self.undo_cursor).unwrap();
            // println!("Restored state from {} seconds ago", ui.time() - restored_state.0);
            state.user.graph = serde_json::from_str(&restored_state.1).unwrap();
            state.user.graph.zoom_level = zoom_level;
            state.user.graph.push_positions_to_imnodes();
            self.graph_edited = true;
        }
    }

    pub fn render(&mut self, ui: &Ui<'_>, size: [f32; 2], state: &mut State, executor: &super::Executor) -> Option<SceneRectangle> {
        // create main window
        let window_begun = Window::new(im_str!("Rust window"))
            .no_decoration()
            .menu_bar(true)
            .movable(false)
            .size(size, Condition::Always)
            .position([0.0, 0.0], Condition::Always)
            .begin(ui);

        let mut requested_scene_rectangle = None;

        if let Some(window_token) = window_begun {
            // menu bar
            if let Some(menu_bar_token) = ui.begin_menu_bar() {
                ui.menu(im_str!("File"), true, || {
                    if MenuItem::new(im_str!("New")).build(ui) {
                        if self.graph_edited {
                            file_io::async_confirm_new(self.winit_proxy.clone(), executor);
                        } else {
                            state.new_file();
                        }
                    }
                    ui.separator();
                    if MenuItem::new(im_str!("Open")).build(ui) {
                        if self.graph_edited {
                            file_io::async_confirm_open(self.winit_proxy.clone(), executor);
                        } else {
                            file_io::async_pick_open(self.winit_proxy.clone(), executor);
                        }
                    }
                    if MenuItem::new(im_str!("Save")).build(ui) {
                        file_io::async_pick_save(self.winit_proxy.clone(), executor);
                    }
                    ui.separator();
                    if MenuItem::new(im_str!("Export scene")).build(ui) {
                        file_io::async_pick_png(self.winit_proxy.clone(), executor);
                    }
                    ui.separator();
                    if MenuItem::new(im_str!("Exit")).build(ui) {
                        if self.graph_edited {
                            file_io::async_confirm_exit(self.winit_proxy.clone(), executor);
                        } else {
                            use crate::CustomEvent;
                            self.winit_proxy.send_event(CustomEvent::RequestExit).unwrap();
                        }
                    }
                });
                if MenuItem::new(im_str!("About")).build(ui) {
                    println!("\"About\" entry clicked");
                }
                menu_bar_token.end(ui);
            }

            // main tabs for graph, rendering and settings
            let tab_bar_begun = TabBar::new(im_str!("main tab bar"))
                .begin(ui);


            if let Some(tab_bar_token) = tab_bar_begun {

                // NODE EDITOR TAB LOGIC
                let force_selected;
                if self.opened_tab[0] {
                    self.opened_tab[0] = false;
                    force_selected = TabItemFlags::SET_SELECTED;
                } else {
                    force_selected = TabItemFlags::empty();
                }
                let node_tab_token = TabItem::new(im_str!("Node editor"))
                    .flags(force_selected)
                    .begin(ui);
                if let Some(token) = node_tab_token {
                    self.render_editor_tab(ui, state);
                    token.end(ui);
                }

                // SCENE VISUALIZATION TAB LOGIC
                let force_selected;
                if self.opened_tab[1] {
                    self.opened_tab[1] = false;
                    force_selected = TabItemFlags::SET_SELECTED;
                } else {
                    force_selected = TabItemFlags::empty();
                }
                let scene_tab_token = TabItem::new(im_str!("Scene"))
                    .flags(force_selected)
                    .begin(ui);
                if let Some(token) = scene_tab_token {
                    requested_scene_rectangle = Some(self.render_scene_tab(ui, state));
                    token.end(ui);
                }

                // SETTINGS TAB LOGIC
                let force_selected;
                if self.opened_tab[2] {
                    self.opened_tab[2] = false;
                    force_selected = TabItemFlags::SET_SELECTED;
                } else {
                    force_selected = TabItemFlags::empty();
                }
                let settings_tab_token = TabItem::new(im_str!("Settings"))
                    .flags(force_selected)
                    .begin(ui);
                if let Some(token) = settings_tab_token {
                    self.render_settings_tab(ui, state);
                    token.end(ui);
                }

                tab_bar_token.end(ui);
            }
            window_token.end(ui);
        }
        requested_scene_rectangle
    }

    fn render_editor_tab(&mut self, ui: &Ui<'_>, state: &mut State) {
        if ui.button(im_str!("Generate Scene"), [0.0, 0.0]) {
            let processing_succesful = state.process_user_state();
            if processing_succesful {
                self.opened_tab[1] = true;
            }
        }
        ui.same_line(0.0);
        if ui.button(im_str!("Undo"), [0.0, 0.0]) {
            self.issue_undo(state, ui.time());
        }
        ui.same_line(0.0);
        if ui.button(im_str!("Redo"), [0.0, 0.0]) {
            self.issue_redo(state);
        }

        if cfg!(feature = "show-timestamps") {
            use chrono::TimeZone;
            let file_info = im_str!("Created: {}; edited: {}; rn: {:X}",
                chrono::Utc.timestamp(state.time_stamps.fc, 0).to_string(),
                chrono::Utc.timestamp(state.time_stamps.fs, 0).to_string(),
                state.time_stamps.vn,
            );
            ui.same_line(0.0);
            ui.text(file_info);
        }

        ui.columns(2, im_str!("editor columns"), false);
        ui.set_current_column_width(120.0);
        // the following code is similar to what a Vec::drain_filter would do,
        // but operates on 2 vectors at the same time.
        let mut i = 0;
        let globals_names = &mut state.user.globals_names;
        let globals_init_values = &mut state.user.globals_init_values;
        while i != globals_names.len() {
            // to make each variable unique, we are gonna push an ID
            let id_token = ui.push_id(i as i32);
            ui.set_next_item_width(80.0);
            ui.text(&globals_names[i]);
            ui.same_line(0.0);
            // this is safe because there is no way that the user clicks two buttons in a single
            // frame
            if ui.small_button(im_str!("X")) {
                globals_init_values.remove(i);
                globals_names.remove(i);
            } else {
                Drag::new(im_str!(""))
                    .speed(0.01)
                    .build(ui, &mut globals_init_values[i]);
                i += 1;
            }
            id_token.pop(ui);
        }
        ui.text(im_str!("add new variable:"));
        ui.set_next_item_width(75.0);
        InputText::new(ui, im_str!("##new_var_input"), &mut self.new_global_buffer)
            .resize_buffer(false)
            .build();
        ui.same_line(0.0);
        if ui.button(im_str!("New"), [0.0, 0.0]) { // TODO: we need a check: the name must be valid!
            let new_name = self.new_global_buffer.to_string();
            use crate::computable_scene::globals::Globals;
            if Globals::valid_name(&new_name) {
                globals_names.push(self.new_global_buffer.to_string());
                globals_init_values.push(0.0);
                self.new_global_buffer.clear();
            }
        }

        ui.next_column();
        let io = ui.io();
        let editor_ne_point = ui.cursor_pos();
        let relative_pos = [io.mouse_pos[0] - editor_ne_point[0], io.mouse_pos[1] - editor_ne_point[1]];
        // detect if the mouse is in the correct area for zoom interaction
        let enable_zoom_interaction: bool = ui.is_mouse_hovering_rect(ui.cursor_pos(), ui.content_region_max());
        if enable_zoom_interaction {
            // handling of graph zoom is a bit tricky because the mouse wheel is continuous but
            // the zoom levels are discretized. first, add the zoom delta for the current frame
            self.accumulated_zoom += self.added_zoom;
            // then check if the accumulated zoom passed a given threshold, and if it did
            // then we zoom up/down the graph and reset the accumulated value.
            // This prevents us from jumping across multiple levels in a single zoom action, which is
            // good because a single mouse wheel scroll can report a huge delta.
            if self.accumulated_zoom < -1.0 {
                state.user.graph.zoom_down_graph(relative_pos);
                self.accumulated_zoom = 0.0;
            }
            if self.accumulated_zoom > 1.0 {
                state.user.graph.zoom_up_graph(relative_pos);
                self.accumulated_zoom = 0.0;
            }
        }
        // regardless of interaction, reset the added_zoom variable
        self.added_zoom = 0.0;
        // run the rendering
        let requested_savestate = state.user.graph.render(ui, &self.availables, &self.graph_fonts);

        if let Some(requested_stamp) = requested_savestate {
            // first, get the timestamp for the last savestate. This is because if the user only moves some nodes around
            // but changes nothing, the requested stamp will remain the same as the last in the stack, it does not matter
            // at which savestate the user currently is.
            let last_stamp = self.undo_stack.back().unwrap().0;
            // directly comparing floats in this case is fine
            #[allow(clippy::float_cmp)]
            if requested_stamp != last_stamp {
                self.issue_savestate(state, ui.time());
            }
        }

        ui.columns(1, im_str!("editor columns"), false);
    }

    fn render_scene_tab(&mut self, ui: &Ui<'_>, state: &mut State) -> SceneRectangle {
        ui.columns(2, im_str!("scene columns"), false);
        ui.set_current_column_width(120.0);
        ui.text("Global variables");

        // and add the UI for updating them
        let width_token = ui.push_item_width(80.0);
        let zip_iter = state.app.computable_scene.globals.get_variables_iter();
        let mut requested_cursor = MouseCursor::Arrow;
        for (name, value) in zip_iter {
            // to make each slider unique, we are gonna push an invisible unique imgui label
            let imgui_name = ImString::new("##".to_string() + name);
            ui.text(name);
            Drag::new(&imgui_name)
                .speed(0.02)
                .build(ui, value);

            if ui.is_item_hovered() {
                requested_cursor = MouseCursor::ResizeEW;
            }
        }
        let available_region = ui.content_region_avail();
        if available_region[1] > 250.0 {
            let y_spacing = available_region[1] - 120.0;
            ui.dummy([1.0, y_spacing]);
        }
        ui.text("Selected object:");
        // TODO: maybe you want to do something different if the user deletes the node and then goes back to the scene
        let node_name = if let Some(block_id) = self.selected_object {
                if let Some(node) = state.user.graph.get_node(block_id) {
                    node.title.clone()
                } else {
                    String::from("<deleted>")
                }
            } else {
                String::new()
            };
        ui.text(ImString::from(node_name));
        ui.set_mouse_cursor(Some(requested_cursor));
        width_token.pop(ui);
        ui.next_column();
        // the scene shall use the whole remaining content space available
        let scene_pos = ui.cursor_pos();
        let available_region = ui.content_region_avail();

        ImageButton::new(self.scene_texture_id, available_region)
            .frame_padding(0)
            .build(ui);
        if ui.is_item_activated() {
            self.winit_proxy.send_event(super::CustomEvent::MouseFreeze).unwrap();
        }
        if ui.is_item_deactivated() {
            self.winit_proxy.send_event(super::CustomEvent::MouseThaw).unwrap();
        }
        if ui.is_item_active() && ui.is_mouse_double_clicked(MouseButton::Left) {
            let clicked_object = state.app.computable_scene.renderer.object_under_cursor(&state.app.manager.device);
            if clicked_object == self.selected_object {
                self.selected_object = None;
            } else {
                self.selected_object = clicked_object;
            }
            state.app.computable_scene.renderer.highlight_object(self.selected_object);
        }
        state.app.camera_enabled = ui.is_item_hovered();
        ui.columns(1, im_str!("scene columns"), false);
        SceneRectangle {
            position: scene_pos,
            size: available_region,
        }
    }

    fn render_settings_tab(&mut self, ui: &Ui<'_>, state: &mut State) {
        let sensitivity = &mut state.app.sensitivity;
        ui.text(im_str!("Zoom sensitivity"));
        let width_token = ui.push_item_width(120.0);
        imgui::Slider::new(im_str!("zoom speed for graph"))
            .range(0.25 ..= 4.0)
            .display_format(im_str!("%.2f"))
            .flags(SliderFlags::NO_INPUT)
            .flags(SliderFlags::LOGARITHMIC)
            .build(ui, &mut sensitivity.graph_zoom);
        imgui::Slider::new(im_str!("zoom speed for scene"))
            .range(0.25 ..= 4.0)
            .display_format(im_str!("%.2f"))
            .flags(SliderFlags::NO_INPUT)
            .flags(SliderFlags::LOGARITHMIC)
            .build(ui, &mut sensitivity.scene_zoom);
        ui.text(im_str!("Camera settings"));
        ui.checkbox(im_str!("use orthographic projection"), &mut state.app.camera_ortho);
        ui.checkbox(im_str!("lock camera to vertical position"), &mut state.app.camera_lock_up);
        imgui::Slider::new(im_str!("horizontal sensitivity"))
            .range(0.1 ..= 2.0)
            .display_format(im_str!("%.2f"))
            .flags(SliderFlags::NO_INPUT)
            .build(ui, &mut sensitivity.camera_horizontal);
        imgui::Slider::new(im_str!("vertical sensitivity"))
            .range(0.1 ..= 2.0)
            .display_format(im_str!("%.2f"))
            .flags(SliderFlags::NO_INPUT)
            .build(ui, &mut sensitivity.camera_vertical);
        ui.text(im_str!("Axes and labels"));
        let mut recreate_axes = false;
        recreate_axes |= imgui::Slider::new(im_str!("length of x, y and z axes"))
            .range(0 ..= 16)
            .flags(SliderFlags::NO_INPUT)
            .build(ui, &mut self.axes_length);
        recreate_axes |= imgui::Slider::new(im_str!("size of unit marks along axes"))
            .range(0.0 ..= 0.25)
            .display_format(im_str!("%.3f"))
            .flags(SliderFlags::NO_INPUT)
            .build(ui, &mut self.axes_marks_size);
        recreate_axes |= imgui::Slider::new(im_str!("size of axis labels"))
            .range(0.0 ..= 0.5)
            .display_format(im_str!("%.2f"))
            .flags(SliderFlags::NO_INPUT)
            .build(ui, &mut self.labels_size);
        if recreate_axes {
            // if the axes have length zero, we need to ALSO clear the labels.
            if self.axes_length != 0 {
                state.app.set_wireframe_axes(self.axes_length, self.axes_marks_size);
                state.app.set_axes_labels(self.axes_length, self.labels_size);
            } else {
                state.app.clear_wireframe_axes();
                state.app.clear_axes_labels();
            }
        }
        width_token.pop(ui);
    }
}
