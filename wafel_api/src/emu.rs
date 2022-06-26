use std::{collections::HashMap, sync::Arc};

use wafel_data_path::DataPath;
use wafel_data_type::{Address, Value};
use wafel_layout::{load_sm64_n64_layout, DataLayout};
use wafel_memory::{EmptySymbolLookup, EmuMemory, MemoryRead};

use crate::{
    data_path_cache::DataPathCache, mario::mario_action_names, read_object_hitboxes, read_surfaces,
    simplified_data_type, DataType, Error, ObjectHitbox, SM64Version, Surface,
};

/// An SM64 API that attaches to a running emulator and can read/write to its
/// memory.
///
/// # Example
///
/// ```no_run
/// use wafel_api::{Emu, SM64Version};
///
/// let pid = 4232;
/// let base_address = 0x0050B110;
/// let memory_size = 0x0040_0000;
///
/// let mut emu = Emu::attach(pid, base_address, memory_size, SM64Version::US);
///
/// loop {
///     let holding_l =
///         (emu.read("gControllerPads[0].button").as_int() & emu.constant("L_TRIG").as_int()) != 0;
///
///     if holding_l && emu.read("gMarioState.action") != emu.constant("ACT_FREEFALL") {
///         emu.write("gMarioState.action", emu.constant("ACT_FREEFALL"));
///         emu.write("gMarioState.vel[1]", 50.0.into());
///     }
/// }
/// ```
#[derive(Debug)]
pub struct Emu {
    layout: Arc<DataLayout>,
    memory: EmuMemory,
    symbols_by_address: HashMap<Address, String>,
    data_path_cache: DataPathCache<EmptySymbolLookup>,
}

impl Emu {
    /// Attach to a running emulator
    ///
    /// # Panics
    ///
    /// Panics if attachment fails, probably because there is no running process with the given
    /// PID.
    #[track_caller]
    pub fn attach(
        pid: u32,
        base_address: usize,
        memory_size: usize,
        sm64_version: SM64Version,
    ) -> Self {
        match Self::try_attach(pid, base_address, memory_size, sm64_version) {
            Ok(this) => this,
            Err(error) => panic!("Error:\n  failed to attach to {}:\n  {}\n", pid, error),
        }
    }

    /// Attach to a running emulator
    ///
    /// # Panics
    ///
    /// Returns an error if attachment fails, probably because there is no running process with
    /// the given PID.
    pub fn try_attach(
        pid: u32,
        base_address: usize,
        memory_size: usize,
        sm64_version: SM64Version,
    ) -> Result<Self, Error> {
        let layout = load_sm64_n64_layout(&sm64_version.to_string().to_lowercase())?;
        let layout = Arc::new(layout);

        let memory = EmuMemory::attach(pid, base_address, memory_size)?;

        let symbols_by_address = layout
            .globals
            .iter()
            .filter_map(|(name, value)| {
                value
                    .address
                    .map(|addr| (Address(addr as usize), name.clone()))
            })
            .collect();

        let data_path_cache = DataPathCache::new(&Arc::new(EmptySymbolLookup), &layout);

        Ok(Self {
            layout,
            memory,
            symbols_by_address,
            data_path_cache,
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
        let path = self.data_path_cache.global(path)?;
        let value = path.read(&self.memory)?;
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
        let bytes = self.memory.read_string(address)?;
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
        let path = self.data_path_cache.global(path)?;
        let address = path.address(&self.memory)?;
        Ok(address)
    }

    /// Return the name of the global variable at the given address.
    ///
    /// Returns None if no global variable is at the address.
    pub fn address_to_symbol(&self, address: Address) -> Option<String> {
        self.symbols_by_address.get(&address).cloned()
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
        let path = DataPath::compile(&self.layout, &EmptySymbolLookup, path)?;
        let data_type = path.concrete_type();
        let simplified = simplified_data_type(&self.layout, &data_type)?;
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
        let path = self.data_path_cache.global(path)?;
        path.write(&mut self.memory, value)?;
        Ok(())
    }

    //     /// Create a save state using the current game state.
    //     pub fn save_state(&self) -> SaveState {
    //         let mut slot = self.memory.create_backup_slot();
    //         self.memory.copy_slot(&mut slot, &self.base_slot);
    //         SaveState::new(Arc::clone(&self.id), self.base_slot_frame, slot)
    //     }
    //
    //     /// Load a save state.
    //     ///
    //     /// # Panics
    //     ///
    //     /// Panics if the save state was produced by a different [Emu] instance.
    //     #[track_caller]
    //     pub fn load_state(&mut self, state: &SaveState) {
    //         if let Err(error) = self.try_load_state(state) {
    //             panic!("{}", error);
    //         }
    //     }
    //
    //     /// Load a save state.
    //     ///
    //     /// Returns an error if the save state was produced by a different [Emu] instance.
    //     pub fn try_load_state(&mut self, state: &SaveState) -> Result<(), Error> {
    //         if !Arc::ptr_eq(&self.id, &state.game_id) {
    //             return Err(Error::SaveStateMismatch);
    //         }
    //         self.memory.copy_slot(&mut self.base_slot, &state.slot);
    //         self.base_slot_frame = state.frame;
    //         self.rerecords = self.rerecords.saturating_add(1);
    //         Ok(())
    //     }
    //
    //     /// Return the number of times that a save state has been loaded using this
    //     /// API.
    //     pub fn rerecords(&self) -> u32 {
    //         self.rerecords
    //     }

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

    /// Return a mapping from Mario action values to their name (e.g. `ACT_IDLE`).
    pub fn mario_action_names(&self) -> HashMap<u32, String> {
        mario_action_names(&self.layout)
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
        let surfaces = read_surfaces(&self.memory, &self.data_path_cache)?;
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
        let hitboxes = read_object_hitboxes(&self.memory, &self.layout, &self.data_path_cache)?;
        Ok(hitboxes)
    }
}
