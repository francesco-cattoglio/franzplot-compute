use crate::state::UserState;
use crate::computable_scene::globals::NameValuePair;

pub enum Action {
    ProcessGraph(UserState),
    UpdateGlobals(Vec<NameValuePair>),
}
