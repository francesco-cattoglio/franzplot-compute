use winit::event_loop::EventLoopProxy;

use super::CustomEvent;
use super::Executor;
// TODO: Check if there is proper support for utf-8 under windows.
// TODO: we probably would like to check if we have an open dialog/
// a background thread already already, to prevent the user from
// opening tons of them by accident.
pub fn async_pick_save(event_loop_proxy: EventLoopProxy<CustomEvent>, executor: &Executor) {
    let dialog = rfd::AsyncFileDialog::new()
        .add_filter("Franzplot", &["frzp"])
        .save_file();

    executor.execut(async move {
        let file = dialog.await;
        if let Some(handle) = file {
            event_loop_proxy.send_event(CustomEvent::SaveFile(handle.path().into())).unwrap();
        }
    });
}

pub fn async_pick_png(event_loop_proxy: EventLoopProxy<CustomEvent>, executor: &Executor) {
    let dialog = rfd::AsyncFileDialog::new()
        .add_filter("Png image", &["png"])
        .save_file();

    executor.execut(async move {
        let file = dialog.await;
        if let Some(handle) = file {
            event_loop_proxy.send_event(CustomEvent::ExportPng(handle.path().into())).unwrap();
        }
    });
}

pub fn async_confirm_load(event_loop_proxy: EventLoopProxy<CustomEvent>, executor: &Executor, file_path: std::path::PathBuf) {
    let confirm_load = rfd::AsyncMessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_description("The current file has unsaved changes. Are you sure you want to load this new file?")
        .set_buttons(rfd::MessageButtons::YesNo)
        .show();

    executor.execut(async move {
        let confirmed = confirm_load.await;
        if confirmed {
            event_loop_proxy.send_event(CustomEvent::OpenFile(file_path)).unwrap();
        }
    });
}

pub fn async_dialog_failure(executor: &Executor, error: &'static str) {
    let error_dialog = rfd::AsyncMessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_description(error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();

    executor.execut(async move {
        let _aknowledged = error_dialog.await;
    });
}

pub fn async_confirm_exit(event_loop_proxy: EventLoopProxy<CustomEvent>, executor: &Executor) {
    let confirm_exit = rfd::AsyncMessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_description("The current file has unsaved changes. Are you sure you want to exit?")
        .set_buttons(rfd::MessageButtons::YesNo)
        .show();

    executor.execut(async move {
        let confirmed = confirm_exit.await;
        if confirmed {
            event_loop_proxy.send_event(CustomEvent::RequestExit).unwrap();
        }
    });
}

pub fn async_confirm_new(event_loop_proxy: EventLoopProxy<CustomEvent>, executor: &Executor) {
    let confirm_new = rfd::AsyncMessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_description("The current file has unsaved changes. Are you sure you want to discard changes and create a new file?")
        .set_buttons(rfd::MessageButtons::YesNo)
        .show();

    executor.execut(async move {
        let confirmed = confirm_new.await;
        if confirmed {
            event_loop_proxy.send_event(CustomEvent::NewFile).unwrap();
        }
    });
}

pub fn async_confirm_open(event_loop_proxy: EventLoopProxy<CustomEvent>, executor: &Executor) {
    let confirm_open = rfd::AsyncMessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_description("The current file has unsaved changes. Are you sure you want to discard changes and open a file?")
        .set_buttons(rfd::MessageButtons::YesNo)
        .show();

    executor.execut(async move {
        let confirmed = confirm_open.await;
        if confirmed {
            event_loop_proxy.send_event(CustomEvent::ShowOpenDialog).unwrap();
        }
    });
}

pub fn async_pick_open(event_loop_proxy: EventLoopProxy<CustomEvent>, executor: &Executor) {
    let dialog = rfd::AsyncFileDialog::new()
        .add_filter("Franzplot", &["frzp"])
        .pick_file();

    executor.execut(async move {
        let file = dialog.await;
        if let Some(handle) = file {
            event_loop_proxy.send_event(CustomEvent::OpenFile(handle.path().into())).unwrap();
        }
    });
}
