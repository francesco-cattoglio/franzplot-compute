use egui::TextureId;
pub use ferre_gui::{FerreGui, FerreData};

use crate::state::{UserState, AppState};
mod ferre_gui;

pub trait Gui {
    fn show(&mut self, ctx: &egui::Context, app_state: &mut AppState, user_state: &mut UserState, id:TextureId);
    fn load_ferre_data(&mut self, ferre_data: FerreData);
    fn export_ferre_data(&self) -> Option<FerreData>;
    fn compute_scene_size(&self) -> Option<wgpu::Extent3d>;
}
