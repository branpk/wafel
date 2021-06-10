use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use wafel_data_path::{DataPathError, GlobalDataPath};
use wafel_data_type::{Address, Value};
use wafel_layout::{DataLayout, DllLayout};
use wafel_memory::{DllGameMemory, GameMemory, MemoryRead};

use crate::Error;

/// An SM64 API that uses a traditional frame advance / save state model.
///
/// See [crate level docs](crate) for more details on which API to use.
///
/// # Example
///
/// ```
/// use wafel_api::Game;
///
/// let mut game = unsafe { Game::open("libsm64/sm64_us.dll") };
///
/// let power_on = game.save_state();
///
/// game.advance_n(1500);
/// assert_eq!(game.read("gCurrLevelNum"), game.constant("LEVEL_BOWSER_1"));
///
/// game.load_state(&power_on);
/// for frame in 0..1000 {
///     if frame % 2 == 1 {
///         game.write("gControllerPads[0].button", game.constant("START_BUTTON"));
///     }
///     game.advance();
/// }
///
/// game.advance_n(500);
/// assert_eq!(
///     game.read("gCurrLevelNum"),
///     game.constant("LEVEL_CASTLE_GROUNDS")
/// );
/// ```
#[derive(Debug)]
pub struct Game {
    id: Arc<()>,
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
            id: Arc::new(()),
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

    /// Read a null terminated string from memory at the given address.
    ///
    /// # Panics
    ///
    /// Panics if reading from memory fails.
    #[track_caller]
    pub fn read_string_at(&self, address: Address) -> Vec<u8> {
        match self.try_read_string_at(address) {
            Ok(bytes) => bytes,
            Err(error) => panic!("Error:\n  failed to read string:\n  {}\n", error),
        }
    }

    /// Read a null terminated string from memory at the given address.
    ///
    /// Returns an error if reading from memory fails.
    pub fn try_read_string_at(&self, address: Address) -> Result<Vec<u8>, Error> {
        let memory = self.memory.with_slot(&self.base_slot);
        let bytes = memory.read_string(address)?;
        Ok(bytes)
    }

    /// Find the address of a path.
    ///
    /// This method returns `None` if `?` is used in the path and the expression before
    /// `?` evaluates to a null pointer.
    ///
    /// See the crate documentation for [wafel_data_path] for the path syntax.
    ///
    /// # Panics
    ///
    /// Panics if the path fails to compile or reading from memory fails.
    #[track_caller]
    pub fn address(&self, path: &str) -> Option<Address> {
        match self.try_address(path) {
            Ok(address) => address,
            Err(error) => panic!("Error:\n  failed to read '{}':\n  {}\n", path, error),
        }
    }

    /// Find the address of a path.
    ///
    /// This method returns `None` if `?` is used in the path and the expression before
    /// `?` evaluates to a null pointer.
    ///
    /// See the crate documentation for [wafel_data_path] for the path syntax.
    ///
    /// Returns an error if the path fails to compile or reading from memory fails.
    pub fn try_address(&self, path: &str) -> Result<Option<Address>, Error> {
        let path = self.data_path(path)?;
        let address = path.address(&self.memory.with_slot(&self.base_slot))?;
        Ok(address)
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
        SaveState::new(Arc::clone(&self.id), self.base_slot_frame, slot)
    }

    /// Load a save state.
    ///
    /// # Panics
    ///
    /// Panics if the save state was produced by a different [Game] instance.
    #[track_caller]
    pub fn load_state(&mut self, state: &SaveState) {
        if let Err(error) = self.try_load_state(state) {
            panic!("{}", error);
        }
    }

    /// Load a save state.
    ///
    /// Returns an error if the save state was produced by a different [Game] instance.
    pub fn try_load_state(&mut self, state: &SaveState) -> Result<(), Error> {
        if !Arc::ptr_eq(&self.id, &state.game_id) {
            return Err(Error::SaveStateMismatch);
        }
        self.memory.copy_slot(&mut self.base_slot, &state.slot);
        self.base_slot_frame = state.frame;
        Ok(())
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

/// A save state used by [Game].
#[derive(Debug)]
pub struct SaveState {
    game_id: Arc<()>,
    frame: u32,
    slot: <DllGameMemory as GameMemory>::Slot,
}

impl SaveState {
    fn new(game_id: Arc<()>, frame: u32, slot: <DllGameMemory as GameMemory>::Slot) -> Self {
        Self {
            game_id,
            frame,
            slot,
        }
    }

    /// Return the frame that the save state was taken on.
    pub fn frame(&self) -> u32 {
        self.frame
    }
}
