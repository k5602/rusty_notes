use super::{Replace, Search};

#[derive(Clone, Debug, PartialEq)]
pub enum EditorState {
    Edit,
    Exit,
    Search(Search),
    Replace(Replace),
}
