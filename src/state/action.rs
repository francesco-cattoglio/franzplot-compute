use std::path::PathBuf;
use crate::state::UserState;
use crate::compute_graph::globals::NameValuePair;

pub enum Action {
    ProcessUserState(),
    WriteToFile(PathBuf),
    OpenFile(PathBuf),
    NewFile(),
    UpdateGlobals(Vec<NameValuePair>),
}
