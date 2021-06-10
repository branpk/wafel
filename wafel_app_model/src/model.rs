use wafel_api::{Address, Timeline, Value};

use crate::range_edit::RangeEdits;

#[derive(Debug)]
pub enum Model {}

impl Model {
    pub fn new() -> Self {}
}

// TODO: data-variables (probably groups & labels as well) should be defined in this crate

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Variable {
    Global(GlobalVariable),
    ObjectField(ObjectVariable, usize),
    SurfaceField(SurfaceVariable, usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GlobalVariable {
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ObjectVariable {
    path: String,
    behavior: Option<Address>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SurfaceVariable {
    path: String,
}
