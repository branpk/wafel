use std::{collections::HashMap, sync::Arc};

use wafel_data_access::{DataPath, MemoryLayout, MemoryLayoutImpl};
use wafel_data_type::{Address, Value};
use wafel_layout::load_sm64_n64_layout;
use wafel_memory::{EmptySymbolLookup, EmuMemory, MemoryRead};
use wafel_sm64::{mario_action_names, read_object_hitboxes, read_surfaces, ObjectHitbox, Surface};
use wafel_viz::{viz_render, VizConfig, VizRenderData};

use crate::{simplified_data_type, DataType, Error, SM64Version};

// TODO: Add to python API:
// - render

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
///
/// let mut emu = Emu::attach(pid, base_address, SM64Version::US);
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
    pub layout: MemoryLayoutImpl<EmptySymbolLookup>, // FIXME: Remove pubs
    pub memory: EmuMemory,
}

impl Emu {
    /// Attach to a running emulator.
    ///
    /// # Panics
    ///
    /// Panics if attachment fails, probably because there is no running process with the given
    /// PID.
    #[track_caller]
    pub fn attach(pid: u32, base_address: usize, sm64_version: SM64Version) -> Self {
        match Self::try_attach(pid, base_address, sm64_version) {
            Ok(this) => this,
            Err(error) => panic!("Error:\n  failed to attach to {}:\n  {}\n", pid, error),
        }
    }

    /// Attach to a running emulator.
    ///
    /// Returns an error if attachment fails, probably because there is no running process with
    /// the given PID.
    pub fn try_attach(
        pid: u32,
        base_address: usize,
        sm64_version: SM64Version,
    ) -> Result<Self, Error> {
        const SM64_MEMORY_SIZE: usize = 0x0040_0000;

        let data_layout = load_sm64_n64_layout(&sm64_version.to_string().to_lowercase())?;
        let data_layout = Arc::new(data_layout);

        let memory = EmuMemory::attach(pid, base_address, SM64_MEMORY_SIZE)?;

        let layout = MemoryLayoutImpl::new(
            &data_layout,
            &Arc::new(EmptySymbolLookup),
            memory.pointer_int_type().size(),
        );

        Ok(Self { layout, memory })
    }

    /// Return true if a process with the given pid is currently open.
    ///
    /// If the process is closed, then reads and writes on this memory object
    /// will fail. Once this method returns false, you should avoid using this
    /// [Emu] again since a new process may eventually open with the same
    /// pid.
    ///
    /// Note that a process may close immediately after calling this method,
    /// so failed reads/writes must be handled regardless.
    pub fn is_process_open(&self) -> bool {
        self.memory.is_process_open()
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
        let path = self.layout.global_path(path)?;
        let address = path.address(&self.memory)?;
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
        let path = DataPath::compile(&self.layout, path)?;
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
        let path = self.layout.global_path(path)?;
        path.write(&mut self.memory, value)?;
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
        let value = self.layout.data_layout().constant(name)?;
        Ok(value.value.into())
    }

    /// Render the game to a [VizRenderData] object, which can be displayed using
    /// [wafel_viz].
    ///
    /// # Panics
    ///
    /// Panics if rendering fails (most likely a bug in [wafel_viz]).
    pub fn render(&self, config: &VizConfig) -> VizRenderData {
        match self.try_render(config) {
            Ok(render_data) => render_data,
            Err(error) => panic!("Error:\n  failed to render:\n  {}\n", error),
        }
    }

    /// Render the game to a [VizRenderData] object, which can be displayed using
    /// [wafel_viz].
    ///
    /// Returns an error if rendering fails (most likely a bug in [wafel_viz]).
    pub fn try_render(&self, config: &VizConfig) -> Result<VizRenderData, Error> {
        let render_data = viz_render(&self.layout, &self.memory, config)?;
        Ok(render_data)
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
        let surfaces = read_surfaces(&self.layout, &self.memory)?;
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
        let hitboxes = read_object_hitboxes(&self.layout, &self.memory)?;
        Ok(hitboxes)
    }
}
