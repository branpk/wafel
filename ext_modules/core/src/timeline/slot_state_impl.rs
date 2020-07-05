use super::{SlotState, SlotStateMut, State};
use crate::{
    data_path::AsGlobalDataPath,
    error::Error,
    memory::{Memory, Value},
};
use std::{borrow::Borrow, ops::DerefMut};

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

    fn address(
        &self,
        path: impl AsGlobalDataPath,
    ) -> Result<<Self::Memory as Memory>::Address, Error> {
        path.as_global_data_path(self.memory)?
            .borrow()
            .address(self.memory, &*self.slot)
    }

    fn read(&self, path: impl AsGlobalDataPath) -> Result<Value, Error> {
        path.as_global_data_path(self.memory)?
            .borrow()
            .read(self.memory, &*self.slot)
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
    fn write(&mut self, path: impl AsGlobalDataPath, value: &Value) -> Result<(), Error> {
        path.as_global_data_path(self.memory)?
            .borrow()
            .write(self.memory, &mut *self.slot, value)
    }
}
