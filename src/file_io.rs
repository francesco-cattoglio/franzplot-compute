use nfd2::Response;
use winit::event_loop::EventLoopProxy;

use super::CustomEvent;
// TODO: Check if there is proper support for utf-8 under windows.
// and if we can get an updated C lib, there are important fixes.
// Also, see if save file filters are handled correctly on every platform.
// If not, maybe find another equivalent library to do dialogs.
// Possible alternatives:
// - wrap https://github.com/mlabbe/nativefiledialog
// - wrap https://github.com/AndrewBelt/osdialog
// - wait for https://github.com/balthild/native-dialog-rs to add support for save dialogs
// - embed a kdialog-like application which handles save file filters correctly in your binary,
//   unpackage it at a temp location and use std::process::Command to run it.
fn show_save_dialog(proxy: EventLoopProxy<CustomEvent>) {
    //if let Some(file_path) = dialog_result {
        //if !filename.is_empty() {
        //    dbg!(&filename);
        //    let mut file_path = std::path::PathBuf::from(filename);
        //    file_path.set_extension("frzp");
        //    proxy.send_event(CustomEvent::SaveFile(file_path)).unwrap();
        //}
    //}
    // if the user cancelled the dialog, do nothing
}

fn show_open_dialog(proxy: EventLoopProxy<CustomEvent>) {
    //match nfd2::open_file_dialog(None, None).expect("oh no") {
    //    Response::Okay(file_path) => println!("File path = {:?}", file_path),
    //    Response::OkayMultiple(files) => println!("Files {:?}", files),
    //    Response::Cancel => println!("User canceled"),
    //}
    //if let Some(file_path) = dialog_result {
    //    if !file_path.exists() {
    //        dbg!(&file_path);
    //        proxy.send_event(CustomEvent::OpenFile(file_path)).unwrap();
    //    }
    //}
    // if the user cancelled the dialog, do nothing
}

// TODO: background threads are probably not needed under OS X
// TODO: we probably would like to check if we have an open dialog/
// a background thread already already, to prevent the user from
// opening tons of them by accident.
pub fn background_file_save(proxy: EventLoopProxy<CustomEvent>) {
    // start a new thread!
    std::thread::spawn(move || {
        show_save_dialog(proxy);
    });
}

pub fn background_file_open(proxy: EventLoopProxy<CustomEvent>, executor: &super::Executor) {
    let dialog = rfd::AsyncFileDialog::new()
        .add_filter("Franzplot", &["frzp"])
        .pick_file();

    let event_loop_proxy = proxy.clone();
        executor.execut(async move {
            let file = dialog.await;
            if let Some(handle) = file {
                event_loop_proxy.send_event(CustomEvent::OpenFile(handle.path().into()));
            }
        });
    // start a new thread!
    //std::thread::spawn(move || {
    //});
}
