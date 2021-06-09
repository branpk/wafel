use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use wafel_data_path::{DataPathError, GlobalDataPath};
use wafel_data_type::Value;
use wafel_layout::{DataLayout, DllLayout};
use wafel_memory::{DllGameMemory, GameMemory};

use crate::Error;

#[derive(Debug)]
pub struct Game {
    layout: Arc<DataLayout>,
    memory: DllGameMemory,
    base_slot_frame: u32,
    base_slot: <DllGameMemory as GameMemory>::Slot,
    data_path_cache: Mutex<HashMap<String, Arc<GlobalDataPath>>>,
}

impl Game {
    /// Load a libsm64 DLL.
    ///
    /// # Panics
    ///
    /// Panics if the DLL fails to open, probably because the file doesn't exist or the DLL
    /// isn't a proper libsm64 DLL.
    ///
    /// # Safety
    ///
    /// This method is inherently unsafe. See the documentation for
    /// [DllGameMemory::load].
    #[track_caller]
    pub unsafe fn open(dll_path: &str) -> Self {
        match Self::try_open(dll_path) {
            Ok(timeline) => timeline,
            Err(error) => panic!("Error:\n  failed to open {}:\n  {}\n", dll_path, error),
        }
    }

    /// Load a libsm64 DLL.
    ///
    /// Returns an error if the DLL fails to open, probably because the file doesn't exist or
    /// the DLL isn't a proper libsm64 DLL.
    ///
    /// # Safety
    ///
    /// This method is inherently unsafe. See the documentation for
    /// [DllGameMemory::load].
    pub unsafe fn try_open(dll_path: &str) -> Result<Self, Error> {
        let mut layout = DllLayout::read(&dll_path)?.data_layout;
        layout.add_sm64_extras()?;
        let layout = Arc::new(layout);

        let (memory, base_slot) = DllGameMemory::load(dll_path, "sm64_init", "sm64_update")?;

        Ok(Self {
            layout,
            memory,
            base_slot_frame: 0,
            base_slot,
            data_path_cache: Mutex::new(HashMap::new()),
        })
    }

    /// Read a value from memory.
    ///
    /// See the crate documentation for [wafel_data_path] for the path syntax.
    ///
    /// # Panics
    ///
    /// Panics if the path fails to compile or reading from memory fails.
    #[track_caller]
    pub fn read(&self, path: &str) -> Value {
        match self.try_read(path) {
            Ok(value) => value,
            Err(error) => panic!("Error:\n  failed to read '{}':\n  {}\n", path, error),
        }
    }

    /// Read a value from memory.
    ///
    /// See the crate documentation for [wafel_data_path] for the path syntax.
    ///
    /// Returns an error if the path fails to compile or reading from memory fails.
    pub fn try_read(&self, path: &str) -> Result<Value, Error> {
        let path = self.data_path(path)?;
        let value = path.read(&self.memory.with_slot(&self.base_slot))?;
        Ok(value)
    }

    /// Write a value to memory.
    ///
    /// See the crate documentation for [wafel_data_path] for the path syntax.
    ///
    /// # Panics
    ///
    /// Panics if the data path fails to compile or the write fails.
    #[track_caller]
    pub fn write(&mut self, path: &str, value: Value) {
        match self.try_write(path, value) {
            Ok(()) => {}
            Err(error) => panic!("Error:\n  failed to write '{}':\n  {}\n", path, error),
        }
    }

    /// Write a value to memory.
    ///
    /// See the crate documentation for [wafel_data_path] for the path syntax.
    ///
    /// Returns an error if the data path fails to compile or the write fails.
    pub fn try_write(&mut self, path: &str, value: Value) -> Result<(), Error> {
        let path = self.data_path(path)?;
        path.write(&mut self.memory.with_slot_mut(&mut self.base_slot), value)?;
        Ok(())
    }

    fn data_path(&self, source: &str) -> Result<Arc<GlobalDataPath>, DataPathError> {
        let mut cache = self.data_path_cache.lock().unwrap();
        match cache.get(source) {
            Some(path) => Ok(Arc::clone(path)),
            None => {
                let path = Arc::new(GlobalDataPath::compile(&self.layout, &self.memory, source)?);
                cache.insert(source.to_string(), path.clone());
                Ok(path)
            }
        }
    }

    /// Get the frame of the current game state.
    pub fn frame(&self) -> u32 {
        self.base_slot_frame
    }

    /// Advance a single frame.
    pub fn advance(&mut self) {
        self.memory.advance_base_slot(&mut self.base_slot);
        self.base_slot_frame += 1;
    }

    /// Advance multiple frames.
    pub fn advance_n(&mut self, num_frames: u32) {
        for _ in 0..num_frames {
            self.advance();
        }
    }

    /// Create a save state using the current game state.
    pub fn save_state(&self) -> SaveState {
        let mut slot = self.memory.create_backup_slot();
        self.memory.copy_slot(&mut slot, &self.base_slot);
        SaveState::new(self.base_slot_frame, slot)
    }

    /// Load a save state.
    pub fn load_state(&mut self, state: &SaveState) {
        self.memory.copy_slot(&mut self.base_slot, &state.slot);
        self.base_slot_frame = state.frame;
    }

    /// Return the value of the macro constant or enum variant with the given name.
    ///
    /// # Panics
    ///
    /// Panics if the constant doesn't exist.
    /// Unless the name has a typo, it is likely that either Wafel is out of date or it is just
    /// a limitation of how Wafel obtains constants from the source.
    #[track_caller]
    pub fn constant(&self, name: &str) -> Value {
        match self.try_constant(name) {
            Ok(value) => value,
            Err(error) => panic!("Error:\n  {}\n", error),
        }
    }

    /// Return the value of the macro constant or enum variant with the given name.
    ///
    /// Returns an error if the constant doesn't exist.
    /// Unless the name has a typo, it is likely that either Wafel is out of date or it is just
    /// a limitation of how Wafel obtains constants from the source.
    pub fn try_constant(&self, name: &str) -> Result<Value, Error> {
        let value = self.layout.constant(name)?;
        Ok(value.value.into())
    }
}

#[derive(Debug)]
pub struct SaveState {
    frame: u32,
    slot: <DllGameMemory as GameMemory>::Slot,
}

impl SaveState {
    fn new(frame: u32, slot: <DllGameMemory as GameMemory>::Slot) -> Self {
        Self { frame, slot }
    }

    /// Return the frame that the save state was taken on.
    pub fn frame(&self) -> u32 {
        self.frame
    }
}
