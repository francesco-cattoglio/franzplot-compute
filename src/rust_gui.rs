use imgui::*;
use crate::file_io;
use crate::state::State;
use crate::computable_scene::globals::Globals;

pub struct Gui {
    pub scene_texture_id: TextureId,
    pub new_global_buffer: ImString,
    winit_proxy: winit::event_loop::EventLoopProxy<super::CustomEvent>,
}

impl Gui {
    pub fn new(scene_texture_id: TextureId, winit_proxy: winit::event_loop::EventLoopProxy<super::CustomEvent>) -> Self {
        Self {
            new_global_buffer: ImString::with_capacity(8),
            scene_texture_id,
            winit_proxy,
        }
    }

    pub fn render(&mut self, ui: &Ui<'_>, size: [f32; 2], state: &mut State) {
        // create main window
        let window_begun = Window::new(im_str!("Rust window"))
            .no_decoration()
            .menu_bar(true)
            .movable(false)
            .size(size, Condition::Always)
            .position([0.0, 0.0], Condition::Always)
            .begin(ui);

        if let Some(window_token) = window_begun {
            // menu bar
            if let Some(menu_bar_token) = ui.begin_menu_bar() {
                ui.menu(im_str!("File"), true, || {
                    if MenuItem::new(im_str!("Save")).build(ui) {
                        println!("save file entry clicked");
                        file_io::background_file_save(self.winit_proxy.clone());
                    }
                    if MenuItem::new(im_str!("Open")).build(ui) {
                        println!("open file entry clicked");
                        file_io::background_file_open(self.winit_proxy.clone());
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
                TabItem::new(im_str!("Node editor"))
                    .build(ui, || {
                        self.render_editor_tab(ui, state);
                    });

                TabItem::new(im_str!("Scene"))
                    .build(ui, || {
                        self.render_scene_tab(ui, state);
                    });

                TabItem::new(im_str!("Settings"))
                    .build(ui, || {
                        self.render_settings_tab(ui);
                    });

                tab_bar_token.end(ui);
            }
            window_token.end(ui);
        }
    }

    fn render_editor_tab(&mut self, ui: &Ui<'_>, state: &mut State) {
        if ui.button(im_str!("Render"), [0.0, 0.0]) {
            state.process_user_state();
        }
        ui.columns(2, im_str!("editor columns"), false);
        ui.set_current_column_width(120.0);
        ui.text(im_str!("Left side"));
        // the following code is similar to what a Vec::drain_filter would do,
        // but operates on 2 vectors at the same time.
        let mut i = 0;
        let globals_names = &mut state.user.globals_names;
        let globals_init_values = &mut state.user.globals_init_values;
        while i != globals_names.len() {
            ui.set_next_item_width(80.0);
            ui.text(&globals_names[i]);
            ui.same_line(0.0);
            if ui.small_button(im_str!("X")) {
                globals_init_values.remove(i);
                globals_names.remove(i);
            } else {
                // to make each slider unique, we are gonna push an invisible unique imgui label
                let imgui_name = ImString::new("##".to_string() + &globals_names[i]);
                Drag::new(&ImString::from(imgui_name))
                    .speed(0.01)
                    .build(ui, &mut globals_init_values[i]);
                i += 1;
            }
        }
        ui.text(im_str!("add new variable:"));
        ui.set_next_item_width(75.0);
        InputText::new(ui, im_str!("##new_var_input"), &mut self.new_global_buffer)
            .resize_buffer(false)
            .build();
        ui.same_line(0.0);
        if ui.button(im_str!("New"), [0.0, 0.0]) { // TODO: we need a check: the name must be valid!
            globals_names.push(self.new_global_buffer.to_string());
            globals_init_values.push(0.0);
            self.new_global_buffer.clear();
        }

        ui.next_column();
        ui.text(im_str!("Right side"));
        state.user.graph.render(ui);
        ui.columns(1, im_str!("editor columns"), false);
    }

    fn render_scene_tab(&self, ui: &Ui<'_>, state: &mut State) {
        ui.columns(2, im_str!("scene columns"), false);
        ui.set_current_column_width(120.0);
        ui.text(im_str!("Globals side"));
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
        ui.set_mouse_cursor(Some(requested_cursor));
        width_token.pop(ui);
        ui.next_column();
        ui.text(im_str!("Scene side"));
        let available_region = ui.content_region_avail();
        ImageButton::new(self.scene_texture_id, available_region)
            .frame_padding(0)
            .build(ui);
        if ui.is_item_active() {
            let mouse_delta = ui.mouse_drag_delta_with_threshold(MouseButton::Left, 0.0);
            ui.reset_mouse_drag_delta(MouseButton::Left);
            state.app.camera_controller.process_mouse(mouse_delta[0], mouse_delta[1]);

        }
        ui.columns(1, im_str!("scene columns"), false);
    }

    fn render_settings_tab(&self, ui: &Ui<'_>) {
        ui.text(im_str!("Setting will appear on this tab"));
    }
}
