use crate::{
    data_path::GlobalDataPath,
    error::Error,
    memory::{Memory, Value},
};

/// An abstract state of the simulation on a given frame.
pub trait State {
    /// The type of memory that the state is taken from.
    type Memory: Memory;

    /// The memory that the state is taken from.
    fn memory(&self) -> &Self::Memory;

    /// The frame of the state.
    fn frame(&self) -> u32;

    /// Get the address for the given path.
    fn address(&self, path: &str) -> Result<<Self::Memory as Memory>::Address, Error> {
        self.address_path(&self.memory().global_path(path)?)
    }

    /// Get the address for the given path.
    fn address_path(
        &self,
        path: &GlobalDataPath,
    ) -> Result<<Self::Memory as Memory>::Address, Error>;

    /// Read from the given path.
    fn read(&self, path: &str) -> Result<Value, Error> {
        self.read_path(&self.memory().global_path(path)?)
    }

    /// Read from the given path.
    fn read_path(&self, path: &GlobalDataPath) -> Result<Value, Error>;
}

/// A state backed by a slot.
pub trait SlotState: State {
    /// The slot that contains the state's content.
    fn slot(&self) -> &<Self::Memory as Memory>::Slot;
}

/// A state backed by a slot and allowing direct memory editing.
pub trait SlotStateMut: SlotState {
    /// The slot that contains the state's content.
    fn slot_mut(&mut self) -> &mut <Self::Memory as Memory>::Slot;

    /// Write to the given path.
    fn write(&mut self, path: &str, value: &Value) -> Result<(), Error> {
        self.write_path(&self.memory().global_path(path)?, value)
    }

    /// Write to the given path.
    fn write_path(&mut self, path: &GlobalDataPath, value: &Value) -> Result<(), Error>;
}
