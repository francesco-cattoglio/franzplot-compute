use imgui::*;
use crate::node_graph;

pub struct Gui {
    pub graph: node_graph::NodeGraph,
    pub scene_texture_id: TextureId,
}

impl Gui {
    pub fn render(&mut self, ui: &Ui<'_>, size: [f32; 2]) {
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
                MenuItem::new(im_str!("File"))
                    .build(ui);
                MenuItem::new(im_str!("About"))
                    .build(ui);

                menu_bar_token.end(ui);
            }

            // main tabs for graph, rendering and settings
            let tab_bar_begun = TabBar::new(im_str!("main tab bar"))
                .begin(ui);
            if let Some(tab_bar_token) = tab_bar_begun {
                TabItem::new(im_str!("Node editor"))
                    .build(ui, || {
                        self.render_editor_tab(ui);
                    });

                TabItem::new(im_str!("Scene"))
                    .build(ui, || {
                        self.render_scene_tab(ui);
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

    fn render_editor_tab(&mut self, ui: &Ui<'_>) {
        ui.columns(2, im_str!("editor columns"), false);
        ui.set_current_column_width(80.0);
        ui.text(im_str!("Left side"));
        ui.next_column();
        ui.text(im_str!("Right side"));
        self.graph.render(ui);
        ui.columns(1, im_str!("editor columns"), false);
    }

    fn render_scene_tab(&self, ui: &Ui<'_>) {
        ui.columns(2, im_str!("scene columns"), false);
        ui.set_current_column_width(80.0);
        ui.text(im_str!("Globals side"));
        ui.next_column();
        ui.text(im_str!("Scene side"));
        let available_region = ui.content_region_avail();
        ImageButton::new(self.scene_texture_id, available_region)
            .frame_padding(0)
            .build(ui);
        ui.columns(1, im_str!("scene columns"), false);
    }

    fn render_settings_tab(&self, ui: &Ui<'_>) {
        ui.text(im_str!("Setting will appear on this tab"));
    }
}
