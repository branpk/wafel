use std::collections::HashMap;

use super::Variable;
use crate::{error::Error, memory::Value};

#[derive(Debug)]
pub struct DirectEdits {
    frames: Vec<HashMap<Variable, Value>>,
}

impl DirectEdits {
    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }

    pub fn insert_frame(&mut self, frame: u32) {
        let index = frame as usize;
        if index < self.frames.len() {
            self.frames.insert(index, HashMap::new());
        }
    }

    pub fn delete_frame(&mut self, frame: u32) {
        let index = frame as usize;
        if index < self.frames.len() {
            self.frames.remove(index);
        }
    }

    pub fn edits(&self, frame: u32) -> impl Iterator<Item = (&Variable, &Value)> {
        self.frames.get(frame as usize).into_iter().flatten()
    }

    fn edits_mut(&mut self, frame: u32) -> &mut HashMap<Variable, Value> {
        let index = frame as usize;
        while self.frames.len() <= index {
            self.frames.push(HashMap::new());
        }
        self.frames.get_mut(index).unwrap()
    }

    pub fn write(&mut self, variable: &Variable, value: Value) -> Result<(), Error> {
        let edits = self.edits_mut(variable.try_frame()?);
        edits.insert(variable.without_frame(), value);
        Ok(())
    }

    pub fn edited(&self, variable: &Variable) -> Result<bool, Error> {
        let frame = variable.try_frame()?;
        let variable = variable.without_frame();
        Ok(self.edits(frame).any(|(var, _)| var == &variable))
    }

    pub fn reset(&mut self, variable: &Variable) -> Result<(), Error> {
        let edits = self.edits_mut(variable.try_frame()?);
        edits.remove(&variable.without_frame());
        Ok(())
    }
}
