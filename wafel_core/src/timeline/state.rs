use crate::{data_path::GlobalDataPath, error::Error, memory::Memory};
use wafel_data_type::{Address, Value};

/// An abstract state of the simulation on a given frame.
pub trait State {
    /// The type of memory that the state is taken from.
    type Memory: Memory;

    /// The memory that the state is taken from.
    fn memory(&self) -> &Self::Memory;

    /// The frame of the state.
    fn frame(&self) -> u32;

    /// Get the address for the given path.
    fn address(&self, path: &str) -> Result<Option<Address>, Error> {
        self.path_address(&self.memory().global_path(path)?)
    }

    /// Get the address for the given path.
    fn path_address(&self, path: &GlobalDataPath) -> Result<Option<Address>, Error>;

    /// Read from the given path.
    fn read(&self, path: &str) -> Result<Value, Error> {
        self.path_read(&self.memory().global_path(path)?)
    }

    /// Read from the given path.
    fn path_read(&self, path: &GlobalDataPath) -> Result<Value, Error>;
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
        self.path_write(&self.memory().global_path(path)?, value)
    }

    /// Write to the given path.
    fn path_write(&mut self, path: &GlobalDataPath, value: &Value) -> Result<(), Error>;
}
