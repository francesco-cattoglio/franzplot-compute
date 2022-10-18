pub use ferre_gui::FerreGui;
mod ferre_gui;


trait Gui {
    fn new() -> Self;
}
