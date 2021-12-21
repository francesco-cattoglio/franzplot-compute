use crate::state::UserState;
use crate::compute_graph::globals::NameValuePair;

pub enum Action {
    ProcessGraph(UserState),
    UpdateGlobals(Vec<NameValuePair>),
}
