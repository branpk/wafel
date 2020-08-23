use super::{SlotState, SlotStateMut, State};
use crate::{
    data_path::GlobalDataPath,
    error::Error,
    memory::{Memory, Value},
};
use std::ops::DerefMut;

#[derive(Debug)]
pub struct SlotStateImpl<'a, M: Memory, S: DerefMut<Target = M::Slot>> {
    pub memory: &'a M,
    pub frame: u32,
    pub slot: S,
}

impl<'a, M: Memory, S: DerefMut<Target = M::Slot>> State for SlotStateImpl<'a, M, S> {
    type Memory = M;

    fn memory(&self) -> &M {
        self.memory
    }

    fn frame(&self) -> u32 {
        self.frame
    }

    fn path_address(
        &self,
        path: &GlobalDataPath,
    ) -> Result<Option<<Self::Memory as Memory>::Address>, Error> {
        path.address(self.memory, &*self.slot)
    }

    fn path_read(&self, path: &GlobalDataPath) -> Result<Value, Error> {
        path.read(self.memory, &*self.slot)
    }
}

impl<'a, M: Memory, S: DerefMut<Target = M::Slot>> SlotState for SlotStateImpl<'a, M, S> {
    /// The slot that contains the state's content.
    fn slot(&self) -> &M::Slot {
        &self.slot
    }
}

impl<'a, M: Memory, S: DerefMut<Target = M::Slot>> SlotStateMut for SlotStateImpl<'a, M, S> {
    /// The slot that contains the state's content.
    fn slot_mut(&mut self) -> &mut M::Slot {
        &mut self.slot
    }

    /// Write to the given path.
    fn path_write(&mut self, path: &GlobalDataPath, value: &Value) -> Result<(), Error> {
        path.write(self.memory, &mut *self.slot, value)
    }
}
