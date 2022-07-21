use std::{collections::HashMap, sync::Arc};

use wafel_data_access::{DataPath, MemoryLayout, MemoryLayoutImpl};
use wafel_data_type::{Address, Value};
use wafel_layout::DllLayout;
use wafel_memory::{DllGameMemory, GameMemory, MemoryRead};

use crate::{
    frame_log::read_frame_log, mario::mario_action_names, read_object_hitboxes, read_surfaces,
    simplified_data_type, DataType, Error, Input, ObjectHitbox, Surface,
};

/// An SM64 API that uses a traditional frame advance / save state model.
///
/// See [crate level docs](crate) for more details on which API to use.
///
/// # Example
///
/// ```no_run
/// use wafel_api::Game;
///
/// let mut game = unsafe { Game::new("libsm64/sm64_us.dll") };
///
/// let power_on = game.save_state();
///
/// game.advance_n(1500);
/// assert_eq!(game.read("gCurrLevelNum"), game.constant("LEVEL_BOWSER_1"));
///
/// game.load_state(&power_on);
/// assert_eq!(game.frame(), 0);
///
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
    layout: MemoryLayoutImpl<DllGameMemory>,
    memory: Arc<DllGameMemory>,
    base_slot_frame: u32,
    base_slot: <DllGameMemory as GameMemory>::Slot,
    rerecords: u32,
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
    /// This method is inherently unsafe:
    /// - If the DLL image is modified (either on disk before load or in memory) from anywhere
    ///   except this [Game], this is UB.
    /// - The DLL can easily execute arbitrary code.
    #[track_caller]
    pub unsafe fn new(dll_path: &str) -> Self {
        match Self::try_new(dll_path) {
            Ok(this) => this,
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
    /// This method is inherently unsafe:
    /// - If the DLL image is modified (either on disk before load or in memory) from anywhere
    ///   except this [Game], this is UB.
    /// - The DLL can easily execute arbitrary code.
    pub unsafe fn try_new(dll_path: &str) -> Result<Self, Error> {
        let mut data_layout = DllLayout::read(&dll_path)?.data_layout;
        data_layout.add_sm64_extras()?;
        let data_layout = Arc::new(data_layout);

        let (memory, base_slot) = DllGameMemory::load(dll_path, "sm64_init", "sm64_update")?;
        let memory = Arc::new(memory);

        let layout = MemoryLayoutImpl::new(&data_layout, &memory);

        Ok(Self {
            id: Arc::new(()),
            layout,
            memory,
            base_slot_frame: 0,
            base_slot,
            rerecords: 0,
        })
    }

    /// Read a value from memory.
    ///
    /// See the [crate documentation](crate) for the path syntax.
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
    /// See the [crate documentation](crate) for the path syntax.
    ///
    /// Returns an error if the path fails to compile or reading from memory fails.
    pub fn try_read(&self, path: &str) -> Result<Value, Error> {
        let path = self.layout.global_path(path)?;
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
    /// See the [crate documentation](crate) for the path syntax.
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
    /// See the [crate documentation](crate) for the path syntax.
    ///
    /// Returns an error if the path fails to compile or reading from memory fails.
    pub fn try_address(&self, path: &str) -> Result<Option<Address>, Error> {
        let path = self.layout.global_path(path)?;
        let address = path.address(&self.memory.with_slot(&self.base_slot))?;
        Ok(address)
    }

    /// Return the name of the global variable at the given address.
    ///
    /// Returns None if no global variable is at the address.
    pub fn address_to_symbol(&self, address: Address) -> Option<String> {
        self.layout.address_to_symbol(address).ok()
    }

    /// Return a simplified description of the type of the given variable.
    ///
    /// See the [crate documentation](crate) for the path syntax.
    ///
    /// # Panics
    ///
    /// Panics if the path fails to compile or type resolution fails.
    #[track_caller]
    pub fn data_type(&self, path: &str) -> DataType {
        match self.try_data_type(path) {
            Ok(data_type) => data_type,
            Err(error) => panic!("Error:\n  {}\n", error),
        }
    }

    /// Return a simplified description of the type of the given variable.
    ///
    /// See the [crate documentation](crate) for the path syntax.
    ///
    /// # Panics
    ///
    /// Panics if the path fails to compile or type resolution fails.
    pub fn try_data_type(&self, path: &str) -> Result<DataType, Error> {
        let path = DataPath::compile(&self.layout.data_layout, &self.memory, path)?;
        let data_type = path.concrete_type();
        let simplified = simplified_data_type(&self.layout.data_layout, &data_type)?;
        Ok(simplified)
    }

    /// Write a value to memory.
    ///
    /// See the [crate documentation](crate) for the path syntax.
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
    /// See the [crate documentation](crate) for the path syntax.
    ///
    /// Returns an error if the data path fails to compile or the write fails.
    pub fn try_write(&mut self, path: &str, value: Value) -> Result<(), Error> {
        let path = self.layout.global_path(path)?;
        path.write(&mut self.memory.with_slot_mut(&mut self.base_slot), value)?;
        Ok(())
    }

    /// Set the game's controller input for the current frame using [Input].
    ///
    /// # Panics
    ///
    /// Panics if the write fails.
    #[track_caller]
    pub fn set_input(&mut self, input: Input) {
        match self.try_set_input(input) {
            Ok(()) => {}
            Err(error) => panic!("Error:\n  failed to set input:\n  {}\n", error),
        }
    }

    /// Set the game's controller input for the current frame using [Input].
    ///
    /// Returns an error if the write fails.
    pub fn try_set_input(&mut self, input: Input) -> Result<(), Error> {
        self.try_write("gControllerPads[0].button", input.buttons.into())?;
        self.try_write("gControllerPads[0].stick_x", input.stick_x.into())?;
        self.try_write("gControllerPads[0].stick_y", input.stick_y.into())?;
        Ok(())
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
        self.rerecords = self.rerecords.saturating_add(1);
        Ok(())
    }

    /// Return the number of times that a save state has been loaded.
    pub fn rerecords(&self) -> u32 {
        self.rerecords
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
        let value = self.layout.data_layout().constant(name)?;
        Ok(value.value.into())
    }

    /// Return a mapping from Mario action values to their name (e.g. `ACT_IDLE`).
    pub fn mario_action_names(&self) -> HashMap<u32, String> {
        mario_action_names(self.layout.data_layout())
    }

    /// Read the Wafel frame log for the previous frame advance.
    ///
    /// # Panics
    ///
    /// Panics if reading the frame log fails, e.g. it contains an invalid event type.
    #[track_caller]
    pub fn frame_log(&self) -> Vec<HashMap<String, Value>> {
        match self.try_frame_log() {
            Ok(frame_log) => frame_log,
            Err(error) => panic!("Error:\n  failed to read frame log:\n  {}\n", error),
        }
    }

    /// Read the Wafel frame log for the previous frame advance.
    ///
    /// Returns an error if reading the frame log fails, e.g. it contains an invalid event type.
    pub fn try_frame_log(&self) -> Result<Vec<HashMap<String, Value>>, Error> {
        let memory = self.memory.with_slot(&self.base_slot);
        let frame_log = read_frame_log(&self.layout, &memory)?;
        Ok(frame_log)
    }

    /// Read the currently loaded surfaces.
    ///
    /// # Panics
    ///
    /// Panics if the read fails.
    #[track_caller]
    pub fn surfaces(&self) -> Vec<Surface> {
        match self.try_surfaces() {
            Ok(surfaces) => surfaces,
            Err(error) => panic!("Error:\n   failed to read surfaces:\n  {}\n", error),
        }
    }

    /// Read the currently loaded surfaces.
    ///
    /// Returns an error if the read fails.
    pub fn try_surfaces(&self) -> Result<Vec<Surface>, Error> {
        let memory = self.memory.with_slot(&self.base_slot);
        let surfaces = read_surfaces(&self.layout, &memory)?;
        Ok(surfaces)
    }

    /// Read the hitboxes for active objects.
    ///
    /// # Panics
    ///
    /// Panics if the read fails.
    #[track_caller]
    pub fn object_hitboxes(&self) -> Vec<ObjectHitbox> {
        match self.try_object_hitboxes() {
            Ok(hitboxes) => hitboxes,
            Err(error) => panic!("Error:\n   failed to read object hitboxes:\n  {}\n", error),
        }
    }

    /// Read the hitboxes for active objects.
    ///
    /// Returns an error if the read fails.
    pub fn try_object_hitboxes(&self) -> Result<Vec<ObjectHitbox>, Error> {
        let memory = self.memory.with_slot(&self.base_slot);
        let hitboxes = read_object_hitboxes(&self.layout, &memory)?;
        Ok(hitboxes)
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
