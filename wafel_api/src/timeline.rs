use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use wafel_data_access::{DataPath, GlobalDataPath, MemoryLayout, MemoryLayoutImpl};
use wafel_data_type::{Address, IntType, Value};
use wafel_layout::DllLayout;
use wafel_memory::{DllGameMemory, DllSlotMemoryView, GameMemory, MemoryRead};
use wafel_sm64::{
    mario_action_names, read_frame_log, read_object_hitboxes, read_surfaces, ObjectHitbox, Surface,
};
use wafel_timeline::{GameController, GameTimeline, InvalidatedFrames};
use wafel_viz::{viz_render, VizConfig, VizRenderData};

use crate::{data_cache::DataCache, simplified_data_type, DataType, Error, Input};

/// An SM64 API that allows reads and writes to arbitrary frames without frame advance or
/// save states.
///
/// See [crate level docs](crate) for more details on which API to use.
///
/// # Example
///
/// ```no_run
/// use wafel_api::Timeline;
///
/// let mut timeline = unsafe { Timeline::new("libsm64/sm64_us.dll") };
///
/// assert_eq!(
///     timeline.read(1500, "gCurrLevelNum"),
///     timeline.constant("LEVEL_BOWSER_1")
/// );
///
/// for frame in 0..1000 {
///     if frame % 2 == 1 {
///         timeline.write(
///             frame,
///             "gControllerPads[0].button",
///             timeline.constant("START_BUTTON"),
///         );
///     }
/// }
///
/// assert_eq!(
///     timeline.read(1500, "gCurrLevelNum"),
///     timeline.constant("LEVEL_CASTLE_GROUNDS")
/// );
/// ```
#[derive(Debug)]
pub struct Timeline {
    memory: Arc<DllGameMemory>,
    layout: MemoryLayoutImpl<DllGameMemory>,
    timeline: Mutex<GameTimeline<Arc<DllGameMemory>, Controller>>,
    data_cache: Mutex<DataCache>,
}

impl Timeline {
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
    ///   except this [Timeline], this is UB.
    /// - The DLL can easily execute arbitrary code.
    #[track_caller]
    pub unsafe fn new(dll_path: &str) -> Self {
        match Self::try_new(dll_path) {
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
    /// This method is inherently unsafe:
    /// - If the DLL image is modified (either on disk before load or in memory) from anywhere
    ///   except this [Timeline], this is UB.
    /// - The DLL can easily execute arbitrary code.
    pub unsafe fn try_new(dll_path: &str) -> Result<Self, Error> {
        let mut data_layout = DllLayout::read(&dll_path)?.data_layout;
        data_layout.add_sm64_extras()?;
        let data_layout = Arc::new(data_layout);

        let (memory, base_slot) = DllGameMemory::load(dll_path, "sm64_init", "sm64_update")?;
        let memory = Arc::new(memory);

        let controller = Controller::new();

        let timeline = GameTimeline::new(Arc::clone(&memory), base_slot, controller, 30);
        let timeline = Mutex::new(timeline);

        let layout = MemoryLayoutImpl::new(&data_layout, &memory, IntType::u_ptr_native().size());

        Ok(Self {
            memory,
            layout,
            timeline,
            data_cache: Mutex::new(DataCache::new()),
        })
    }

    /// Set a hotspot with a given name.
    ///
    /// A hotspot is a hint to the algorithm that scrolling should be smooth near the
    /// given frame.
    ///
    /// [balance_distribution](Self::balance_distribution) must also be called frequently
    /// to maintain this.
    pub fn set_hotspot(&mut self, name: &str, frame: u32) {
        self.timeline.lock().unwrap().set_hotspot(name, frame);
    }

    /// Delete a hotspot with the given name, if it exists.
    pub fn delete_hotspot(&mut self, name: &str) {
        self.timeline.lock().unwrap().delete_hotspot(name)
    }

    /// Perform housekeeping to improve scrolling near hotspots.
    pub fn balance_distribution(&mut self, max_run_time_seconds: f32) {
        self.timeline
            .lock()
            .unwrap()
            .balance_distribution(Duration::from_secs_f32(max_run_time_seconds));
    }

    /// Read a value from memory on the given frame.
    ///
    /// See the [crate documentation](crate) for the path syntax.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The path fails to compile
    /// - Reading from memory fails
    /// - A `write` on a previous frame failed
    #[track_caller]
    pub fn read(&self, frame: u32, path: &str) -> Value {
        match self.try_read(frame, path) {
            Ok(value) => value,
            Err(error) => panic!("Error:\n  failed to read '{}':\n  {}\n", path, error),
        }
    }

    fn with_slot_memory<R>(
        &self,
        frame: u32,
        f: impl FnOnce(&DllSlotMemoryView<'_>) -> Result<R, Error>,
    ) -> Result<R, Error> {
        let mut timeline = self.timeline.lock().unwrap();
        let slot = timeline.frame_checked(frame, false)?;
        let slot_memory = self.memory.with_slot(slot);
        f(&slot_memory)
    }

    /// Read a value from memory on the given frame.
    ///
    /// See the [crate documentation](crate) for the path syntax.
    ///
    /// Returns an error if:
    /// - The path fails to compile
    /// - Reading from memory fails
    /// - A `write` on a previous frame failed
    pub fn try_read(&self, frame: u32, path: &str) -> Result<Value, Error> {
        let path = self.layout.global_path(path)?;
        let mut data_cache = self.data_cache.lock().unwrap();

        let value = data_cache.get(frame, &path);
        let value = match value {
            Some(value) => value,
            None => self.read_and_cache(&mut data_cache, frame, &path)?,
        };
        Ok(value)
    }

    fn read_and_cache(
        &self,
        data_cache: &mut DataCache,
        frame: u32,
        path: &Arc<GlobalDataPath>,
    ) -> Result<Value, Error> {
        self.with_slot_memory(frame, |memory| {
            data_cache.preload_frame(frame, memory);

            let value = path.read(memory)?;
            data_cache.insert(frame, path, value.clone());

            Ok(value)
        })
    }

    /// Read a null terminated string from memory on the given frame.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - Reading from memory fails
    /// - A `write` on a previous frame failed
    #[track_caller]
    pub fn read_string_at(&self, frame: u32, address: Address) -> Vec<u8> {
        match self.try_read_string_at(frame, address) {
            Ok(bytes) => bytes,
            Err(error) => panic!("Error:\n  failed to read string:\n  {}\n", error),
        }
    }

    /// Read a null terminated string from memory on the given frame.
    ///
    /// Returns an error if:
    /// - Reading from memory fails
    /// - A `write` on a previous frame failed
    pub fn try_read_string_at(&self, frame: u32, address: Address) -> Result<Vec<u8>, Error> {
        self.with_slot_memory(frame, |memory| Ok(memory.read_string(address)?))
    }

    /// Find the address of a path on the given frame.
    ///
    /// This method returns `None` if `?` is used in the path and the expression before
    /// `?` evaluates to a null pointer.
    ///
    /// See the [crate documentation](crate) for the path syntax.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The path fails to compile
    /// - Reading from memory fails
    /// - A `write` on a previous frame failed
    #[track_caller]
    pub fn address(&self, frame: u32, path: &str) -> Option<Address> {
        match self.try_address(frame, path) {
            Ok(address) => address,
            Err(error) => panic!("Error:\n  failed to read '{}':\n  {}\n", path, error),
        }
    }

    /// Find the address of a path on the given frame.
    ///
    /// This method returns `None` if `?` is used in the path and the expression before
    /// `?` evaluates to a null pointer.
    ///
    /// See the [crate documentation](crate) for the path syntax.
    ///
    /// Returns an error if:
    /// - The path fails to compile
    /// - Reading from memory fails
    /// - A `write` on a previous frame failed
    pub fn try_address(&self, frame: u32, path: &str) -> Result<Option<Address>, Error> {
        let path = self.layout.global_path(path)?;
        self.with_slot_memory(frame, |memory| Ok(path.address(memory)?))
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

    /// Write a value on the given frame.
    ///
    /// See the [crate documentation](crate) for the path syntax.
    ///
    /// # Panics
    ///
    /// Panics if the data path fails to compile.
    /// Note that this method does not panic if there is a write error, e.g. null pointer
    /// dereference.
    /// Instead, write errors will be returned if `read` is called on a frame later than or
    /// equal to `frame`.
    #[track_caller]
    pub fn write(&mut self, frame: u32, path: &str, value: Value) {
        match self.try_write(frame, path, value) {
            Ok(()) => {}
            Err(error) => panic!("Error:\n  failed to write '{}':\n  {}\n", path, error),
        }
    }

    /// Write a value on the given frame.
    ///
    /// See the [crate documentation](crate) for the path syntax.
    ///
    /// Returns an error if the data path fails to compile.
    /// Note that this method does not return an error if there is a write error, e.g. null
    /// pointer dereference.
    /// Instead, write errors will be returned if `read` is called on a frame later than or
    /// equal to `frame`.
    pub fn try_write(&mut self, frame: u32, path: &str, value: Value) -> Result<(), Error> {
        let path = self.layout.global_path(path)?;
        self.with_controller_mut(|controller| controller.write(frame, &path, value));
        Ok(())
    }

    /// Set the game's controller input for the given frame using [Input].
    ///
    /// # Panics
    ///
    /// Panics if path compilation fails.
    #[track_caller]
    pub fn set_input(&mut self, frame: u32, input: Input) {
        match self.try_set_input(frame, input) {
            Ok(()) => {}
            Err(error) => panic!("Error:\n  failed to set input:\n  {}\n", error),
        }
    }

    /// Set the game's controller input for the given frame using [Input].
    ///
    /// Returns an error if path compilation fails.
    pub fn try_set_input(&mut self, frame: u32, input: Input) -> Result<(), Error> {
        self.try_write(frame, "gControllerPads[0].button", input.buttons.into())?;
        self.try_write(frame, "gControllerPads[0].stick_x", input.stick_x.into())?;
        self.try_write(frame, "gControllerPads[0].stick_y", input.stick_y.into())?;
        Ok(())
    }

    /// Clear all previous calls to [write](Self::write) with the given `frame` and `path`.
    ///
    /// The `path` string must match the one passed to `write` exactly. If it is different,
    /// then this method will do nothing.
    ///
    /// # Panics
    ///
    /// Panics if the data path fails to compile.
    #[track_caller]
    pub fn reset(&mut self, frame: u32, path: &str) {
        match self.try_reset(frame, path) {
            Ok(()) => {}
            Err(error) => panic!("Error:\n  failed to reset '{}':\n  {}\n", path, error),
        }
    }

    /// Clear all previous calls to [write](Self::write) with the given `frame` and `path`.
    ///
    /// The `path` string must match the one passed to `write` exactly. If it is different,
    /// then this method will do nothing.
    ///
    /// Returns an error if the data path fails to compile.
    pub fn try_reset(&mut self, frame: u32, path: &str) -> Result<(), Error> {
        let path = self.layout.global_path(path)?;
        self.with_controller_mut(|controller| controller.reset(frame, &path));
        Ok(())
    }

    /// Clear all previous calls to [write](Self::write).
    pub fn reset_all(&mut self) {
        self.with_controller_mut(|controller| controller.reset_all());
    }

    /// Shift all edits to the right, starting at the given frame.
    pub fn insert_frame(&mut self, frame: u32) {
        self.with_controller_mut(|controller| controller.insert_frame(frame));
    }

    /// Delete edits at the given frame, shifting all later edits to the left.
    pub fn delete_frame(&mut self, frame: u32) {
        self.with_controller_mut(|controller| controller.delete_frame(frame));
    }

    fn with_controller_mut(&mut self, func: impl FnOnce(&mut Controller) -> InvalidatedFrames) {
        let data_cache = self.data_cache.get_mut().unwrap();
        let timeline = self.timeline.get_mut().unwrap();

        timeline.with_controller_mut(|controller| {
            let invalidated_frames = func(controller);
            if let InvalidatedFrames::StartingAt(frame) = invalidated_frames {
                data_cache.invalidate_frame(frame);
            }
            invalidated_frames
        });
    }

    /// Return the size of the data cache in bytes.
    pub fn dbg_data_cache_size(&self) -> usize {
        self.data_cache.lock().unwrap().byte_size()
    }

    /// Return a list of the frames that are currently loaded by the algorithm.
    pub fn dbg_cached_frames(&self) -> Vec<u32> {
        self.timeline.lock().unwrap().cached_frames()
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
    pub fn render(&self, frame: u32, config: &VizConfig) -> Result<VizRenderData, Error> {
        match self.try_render(frame, config) {
            Ok(render_data) => Ok(render_data),
            Err(error) => panic!("Error:\n  failed to render:\n  {}\n", error),
        }
    }

    /// Render the game to a [VizRenderData] object, which can be displayed using
    /// [wafel_viz].
    ///
    /// Returns an error if rendering fails (most likely a bug in [wafel_viz]).
    pub fn try_render(&self, frame: u32, config: &VizConfig) -> Result<VizRenderData, Error> {
        self.with_slot_memory(frame, |memory| {
            viz_render(&self.layout, memory, config).map_err(Error::from)
        })
    }

    /// Return a mapping from Mario action values to their name (e.g. `ACT_IDLE`).
    pub fn mario_action_names(&self) -> HashMap<u32, String> {
        mario_action_names(&self.layout)
    }

    /// Read the Wafel frame log for the previous frame advance.
    ///
    /// # Panics
    ///
    /// Panics if reading the frame log fails, e.g. it contains an invalid event type, or if a
    /// `write` on a previous frame failed.
    #[track_caller]
    pub fn frame_log(&self, frame: u32) -> Vec<HashMap<String, Value>> {
        match self.try_frame_log(frame) {
            Ok(frame_log) => frame_log,
            Err(error) => panic!("Error:\n  failed to read frame log:\n  {}\n", error),
        }
    }

    /// Read the Wafel frame log for the previous frame advance.
    ///
    /// Returns an error if reading the frame log fails, e.g. it contains an invalid event type,
    /// or if a `write` on a previous frame failed.
    pub fn try_frame_log(&self, frame: u32) -> Result<Vec<HashMap<String, Value>>, Error> {
        self.with_slot_memory(frame, |memory| {
            read_frame_log(&self.layout, memory).map_err(Error::from)
        })
    }

    /// Read the currently loaded surfaces.
    ///
    /// # Panics
    ///
    /// Panics if the read fails or if a `write` on a previous frame failed.
    #[track_caller]
    pub fn surfaces(&self, frame: u32) -> Vec<Surface> {
        match self.try_surfaces(frame) {
            Ok(surfaces) => surfaces,
            Err(error) => panic!("Error:\n   failed to read surfaces:\n  {}\n", error),
        }
    }

    /// Read the currently loaded surfaces.
    ///
    /// Returns an error if the read fails or if a `write` on a previous frame failed.
    pub fn try_surfaces(&self, frame: u32) -> Result<Vec<Surface>, Error> {
        self.with_slot_memory(frame, |memory| {
            read_surfaces(&self.layout, memory).map_err(Error::from)
        })
    }

    /// Read the hitboxes for active objects.
    ///
    /// # Panics
    ///
    /// Panics if the read fails or if a `write` on a previous frame failed.
    #[track_caller]
    pub fn object_hitboxes(&self, frame: u32) -> Vec<ObjectHitbox> {
        match self.try_object_hitboxes(frame) {
            Ok(hitboxes) => hitboxes,
            Err(error) => panic!("Error:\n   failed to read object hitboxes:\n  {}\n", error),
        }
    }

    /// Read the hitboxes for active objects.
    ///
    /// Returns an error if the read fails or if a `write` on a previous frame failed.
    pub fn try_object_hitboxes(&self, frame: u32) -> Result<Vec<ObjectHitbox>, Error> {
        self.with_slot_memory(frame, |memory| {
            read_object_hitboxes(&self.layout, memory).map_err(Error::from)
        })
    }
}

#[derive(Debug, Clone, Default)]
struct Controller {
    edits: Vec<Vec<(Arc<GlobalDataPath>, Value)>>,
}

impl Controller {
    fn new() -> Self {
        Self::default()
    }

    fn get_mut(&mut self, frame: u32) -> &mut Vec<(Arc<GlobalDataPath>, Value)> {
        let index = frame as usize;
        if index >= self.edits.len() {
            self.edits.resize_with(index + 1, Vec::new);
        }
        &mut self.edits[index]
    }

    fn write(
        &mut self,
        frame: u32,
        data_path: &Arc<GlobalDataPath>,
        value: Value,
    ) -> InvalidatedFrames {
        let frame_edits = self.get_mut(frame);
        frame_edits.retain(|(edit_path, _)| !Arc::ptr_eq(data_path, edit_path));
        frame_edits.push((Arc::clone(data_path), value));
        InvalidatedFrames::StartingAt(frame)
    }

    fn reset(&mut self, frame: u32, data_path: &Arc<GlobalDataPath>) -> InvalidatedFrames {
        if let Some(frame_edits) = self.edits.get_mut(frame as usize) {
            let mut found = false;
            frame_edits.retain(|(edit_path, _)| {
                let matches = Arc::ptr_eq(data_path, edit_path);
                found |= matches;
                !matches
            });
            if found {
                return InvalidatedFrames::StartingAt(frame);
            }
        }
        InvalidatedFrames::None
    }

    fn reset_all(&mut self) -> InvalidatedFrames {
        self.edits.clear();
        InvalidatedFrames::StartingAt(0)
    }

    fn insert_frame(&mut self, frame: u32) -> InvalidatedFrames {
        let index = frame as usize;
        if index < self.edits.len() {
            self.edits.insert(index, Vec::new());
            InvalidatedFrames::StartingAt(frame)
        } else {
            InvalidatedFrames::None
        }
    }

    fn delete_frame(&mut self, frame: u32) -> InvalidatedFrames {
        let index = frame as usize;
        if index < self.edits.len() {
            self.edits.remove(index);
            InvalidatedFrames::StartingAt(frame)
        } else {
            InvalidatedFrames::None
        }
    }
}

impl<M: GameMemory> GameController<M> for Controller {
    type Error = Error;

    fn apply(&self, memory: &M, slot: &mut M::Slot, frame: u32) -> Vec<Self::Error> {
        let mut errors = Vec::new();
        if let Some(frame_edits) = self.edits.get(frame as usize) {
            for (path, value) in frame_edits {
                if let Err(error) = path.write(&mut memory.with_slot_mut(slot), value.clone()) {
                    errors.push(Error::ApplyEditError {
                        path: Arc::clone(path),
                        value: value.clone(),
                        error,
                    });
                }
            }
        }
        errors
    }
}
