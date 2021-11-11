use crate::state::UserState;

pub enum Action {
    ProcessGraph(UserState),
}
