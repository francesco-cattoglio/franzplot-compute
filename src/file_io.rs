use std::io::BufReader;
use std::path::{Path, PathBuf};

use winit::event_loop::EventLoopProxy;
use serde::{Serialize, Deserialize};

use super::CustomEvent;
use crate::gui::FerreData;
use crate::state::user_state::{UserState, UserStateV1, TSs, UserGlobals};
use crate::util::Executor;

// File versioning is a bit of a mess, unfortunately, especially when loading
// an older version. This is because I switched between a Tuple to a struct
// when going from V1 to V2
#[derive(Deserialize, Serialize)]
pub enum VersionV1 {
    V0(UserStateV1), // this should be interpreted as "V1.0"
    V1(UserStateV1, TSs), // this should be interpreted as "V1.1"
}

#[derive(Deserialize, Serialize)]
#[non_exhaustive]
pub enum VersionV2 {
    V20 {
        ferre_data: Option<FerreData>,
        user_state: UserState,
    },
}

#[non_exhaustive]
pub enum File {
    V1(VersionV1),
    V2(VersionV2),
}

impl File {
    pub fn read_from_frzp(path: &Path) -> Result<Self, String> {
        let mut file = std::fs::File::open(path).unwrap();
        let mut contents = String::new();
        use std::io::Read;
        file.read_to_string(&mut contents)
            .map_err(|error| format!("Error opening file: {}", &error))?;

        // try to parse the string as a FileVersionV1
        let maybe_data_v1: Result<VersionV1, ron::error::SpannedError> = ron::from_str(&contents);
        if let Ok(data_v1) = maybe_data_v1 {
            return Ok(File::V1(data_v1));
        }

        // previous parsing failed, try to parse it as a FileVersionV2
        let maybe_data_v2: Result<VersionV2, ron::error::SpannedError> = ron::from_str(&contents);
        if let Ok(data_v2) = maybe_data_v2 {
            return Ok(File::V2(data_v2));
        }

        // all parsing failed, report error to the user
        Err("Error reading file contents. Is this a franzplot file?".to_string())
    }

    pub fn convert_to_v2(self) -> Result<VersionV2, String> {
        match self {
            File::V1(VersionV1::V0(user_state)) => {
                // loading an older file that does NOT have timestamp infos.
                // Destructure the contents of the file
                // and assign them to a more recent version of the UserState
                let UserStateV1 {
                    node_graph,
                    globals_names,
                    globals_init_values,
                } = user_state;

                // return
                Ok(VersionV2::V20 {
                    user_state: UserState {
                        node_graph,
                        globals: UserGlobals {
                            names: globals_names,
                            init_values: globals_init_values,
                        },
                        tss: TSs::new_unknown(),
                    },
                    ferre_data: None,
                })
            }
            File::V1(VersionV1::V1(user_state, tss)) => {
                // if we load a V1, we can just read the time stamps,
                // and put them in the UserState
                let UserStateV1 {
                    node_graph,
                    globals_names,
                    globals_init_values,
                } = user_state;

                // return
                Ok(VersionV2::V20 {
                    user_state: UserState {
                        node_graph,
                        globals: UserGlobals {
                            names: globals_names,
                            init_values: globals_init_values,
                        },
                        tss,
                    },
                    ferre_data: None,
                })
            }
            File::V2(data_v2) => {
                // no need to do any conversion, we can just return the data
                Ok(data_v2)
            }
        }
    }

    pub fn write_to_frzp(self, path: &Path) -> Result<(), String> {
        let mut file = std::fs::File::create(path).unwrap();
        let ser_config = ron::ser::PrettyConfig::new()
            .depth_limit(5)
            .indentor("  ".to_owned())
            .separate_tuple_members(true)
            .enumerate_arrays(true);
        // update the time_stamp to remember the last time the file was saved
        let data_v2 = self.convert_to_v2()?;
        let serialized_data = ron::ser::to_string_pretty(&data_v2, ser_config).unwrap();
        let mut contents = r##"//// FRANZPLOT DATA FILE V2.0 \\\\

//   This file should not be edited by hand,
//   as doing so might easily corrupt the data.
//   To edit this file, open it in Franzplot, version 22.10 or higher

"##.to_string();

        contents.push_str(&serialized_data);
        use std::io::Write;
        file.write_all(contents.as_bytes()).unwrap(); // TODO: handle writing failures
        Ok(())
    }
}

pub fn load_file_part_list(path: &Path) -> Result<Option<Vec<(String, PathBuf)>>, String> {
    let file = std::fs::File::open(path).map_err(|err| { err.to_string() })?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let reader_result = serde_json::from_reader(reader);
    let json_value: serde_json::Value = if let Ok(value) = reader_result {
        value
    } else {
        return Ok(None); // could not parse as JSON, that means this was a frzp file
    };

    let parts_list = json_value.get("parts_list").ok_or_else(|| String::from("The JSON file does not contain the `parts_list` array"))?;
    let parts_array = parts_list.as_array().ok_or_else(|| String::from("The `parts_list` field in the JSON file is not an array"))?;
    let mut to_return = Vec::<(String, PathBuf)>::new();
    for object in parts_array.iter() {
        let map = object.as_object().ok_or_else(|| String::from("Error in JSON array: expected an object"))?;
        if map.len() != 1 {
            return Err(String::from("Error in JSON array: each entry should only have 1 field"));
        }
        for entry in map.iter() {
            // we know for a fact that there is only one entry
            let entry_name = entry.0.clone();
            let file_name = entry.1.as_str().ok_or_else(|| String::from("Error in JSON: all values should be strings"))?;
            let mut path_buf = path.to_path_buf();
            path_buf.set_file_name(file_name);
            to_return.push((entry_name, path_buf));
        }
    }

    Ok(Some(to_return))
}

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
            event_loop_proxy.send_event(CustomEvent::ExportScenePng(handle.path().into())).unwrap();
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

pub fn async_dialog_failure(executor: &Executor, error: String) {
    let error_dialog = rfd::AsyncMessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_description(&error)
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
        .add_filter("Franzplot part list", &["json"])
        .pick_file();

    executor.execut(async move {
        let file = dialog.await;
        if let Some(handle) = file {
            event_loop_proxy.send_event(CustomEvent::OpenFile(handle.path().into())).unwrap();
        }
    });
}
