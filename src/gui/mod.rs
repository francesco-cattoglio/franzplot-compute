use egui::TextureId;
pub use ferre_gui::{FerreGui, FerreData};
pub use node_gui::NodeGui;

use crate::state::{UserState, AppState, Action};
mod ferre_gui;
mod node_gui;

pub trait Gui {
    fn show(&mut self, ctx: &egui::Context, app_state: &mut AppState, user_state: &mut UserState, id:TextureId) -> Option<Action>;
    fn mark_new_file_open(&mut self, ctx: &egui::Context);
    fn mark_new_part_open(&mut self, ctx: &egui::Context);
    fn load_ferre_data(&mut self, ctx: &egui::Context, ferre_data: FerreData);
    fn export_ferre_data(&self) -> Option<FerreData>;
    fn compute_scene_size(&self) -> Option<wgpu::Extent3d>;
}

pub struct Availables {
    pub mask_ids: Vec<egui::TextureId>,
    pub material_ids: Vec<egui::TextureId>,
    pub model_names: Vec<&'static str>,

}
