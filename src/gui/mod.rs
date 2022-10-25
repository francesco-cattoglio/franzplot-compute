use egui::TextureId;
pub use ferre_gui::FerreGui;

use crate::state::{UserState, AppState};
mod ferre_gui;


pub trait Gui {
    fn show(&mut self, raw_input: egui::RawInput, app_state: &mut AppState, user_state: &mut UserState, id:TextureId) -> egui::FullOutput;
    fn compute_scene_size(&self) -> Option<wgpu::Extent3d>;
}
