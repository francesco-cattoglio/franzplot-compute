use crate::state::UserState;

pub struct FerreGui {

}
impl FerreGui {
    pub fn new() -> Self {
        FerreGui {

        }
    }
}

impl super::Gui for FerreGui {
    fn show(&self, ctx: &egui::Context, raw_input: egui::RawInput, user_state: &mut UserState) -> egui::FullOutput {
        ctx.begin_frame(raw_input);

        egui::SidePanel::left("procedure panel").show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .show(ui, |ui| {
                    for (id, node) in user_state.node_graph.get_nodes() {
                        ui.horizontal(|ui| {
                            let node_label = format!("Node {}: {}", id, node.title);
                            ui.label(node_label);
                            if ui.button("test").clicked() {
dbg!("43");
                            }
                        }); // horizontal

                    }
                }); // Scrollable area.
        }); // left panel
egui::Window::new("My Window2")
                .drag_bounds(egui::Rect::EVERYTHING)
                .show(ctx, |ui| {
   ui.label("Hello World!");
});

//let texture_size = wgpu::Extent3d {
//    width: 320,
//    height: 320,
//    ..Default::default()
//};
//let render_request = Action::RenderScene(texture_size, &scene_view);
//state.process(render_request).expect("failed to render the scene due to an unknown error");
                // End the UI frame. We could now handle the output and draw the UI with the backend.
                ctx.end_frame()

    }
}
