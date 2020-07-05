use std::collections::HashMap;

use super::Variable;
use crate::memory::{Memory, Value};

#[derive(Debug)]
pub struct DirectEdits<M: Memory> {
    frames: Vec<HashMap<Variable, Value<M::Address>>>,
}

impl<M: Memory> DirectEdits<M> {
    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }

    pub fn insert_frame(&mut self, frame: u32) {
        let index = frame as usize;
        while self.frames.len() <= index {
            self.frames.push(HashMap::new());
        }
        self.frames.insert(index, HashMap::new());
    }

    pub fn delete_frame(&mut self, frame: u32) {
        let index = frame as usize;
        if index < self.frames.len() {
            self.frames.remove(index);
        }
    }

    pub fn edits(&self, frame: u32) -> impl Iterator<Item = (&Variable, &Value<M::Address>)> {
        self.frames.get(frame as usize).into_iter().flatten()
    }

    fn edits_mut(&mut self, frame: u32) -> &mut HashMap<Variable, Value<M::Address>> {
        let index = frame as usize;
        while self.frames.len() <= index {
            self.frames.push(HashMap::new());
        }
        self.frames.get_mut(index).unwrap()
    }

    pub fn write(&mut self, variable: &Variable, value: Value<M::Address>) {
        let edits = self.edits_mut(variable.frame_unwrap());
        edits.insert(variable.without_frame(), value);
    }

    pub fn edited(&self, variable: &Variable) -> bool {
        let frame = variable.frame_unwrap();
        let variable = variable.without_frame();
        return self.edits(frame).any(|(var, _)| var == &variable);
    }

    pub fn reset(&mut self, variable: &Variable) {
        let edits = self.edits_mut(variable.frame_unwrap());
        edits.remove(&variable.without_frame());
    }
}
