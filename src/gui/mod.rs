pub use ferre_gui::FerreGui;

use crate::state::UserState;
mod ferre_gui;


pub trait Gui {
    fn show(&self, ctx: &egui::Context, raw_input: egui::RawInput, user_state: &mut UserState) -> egui::FullOutput;
}
