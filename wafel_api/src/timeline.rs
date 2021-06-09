use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use wafel_data_path::{DataPathError, GlobalDataPath};
use wafel_data_type::Value;
use wafel_layout::{DataLayout, DllLayout};
use wafel_memory::{DllGameMemory, GameMemory};
use wafel_timeline::{GameController, GameTimeline, InvalidatedFrames};

use crate::{data_cache::DataCache, Error};

pub struct Timeline {
    memory: Arc<DllGameMemory>,
    layout: Arc<DataLayout>,
    timeline: Mutex<GameTimeline<Arc<DllGameMemory>, Controller>>,
    data_path_cache: Mutex<HashMap<String, Arc<GlobalDataPath>>>,
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
        let memory = Arc::new(memory);

        let controller = Controller::new();

        let timeline = GameTimeline::new(Arc::clone(&memory), base_slot, controller, 30);
        let timeline = Mutex::new(timeline);

        Ok(Self {
            memory,
            layout,
            timeline,
            data_path_cache: Mutex::new(HashMap::new()),
            data_cache: Mutex::new(DataCache::new()),
        })
    }

    /// Read a value from memory on the given frame.
    ///
    /// See the crate documentation for [wafel_data_path] for the path syntax.
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

    /// Read a value from memory on the given frame.
    ///
    /// See the crate documentation for [wafel_data_path] for the path syntax.
    ///
    /// Returns an error if:
    /// - The path fails to compile
    /// - Reading from memory fails
    /// - A `write` on a previous frame failed
    pub fn try_read(&self, frame: u32, path: &str) -> Result<Value, Error> {
        let path = self.data_path(path)?;
        let mut timeline = self.timeline.lock().unwrap();
        let mut data_cache = self.data_cache.lock().unwrap();

        let value = match data_cache.get(frame, &path) {
            Some(value) => value,
            None => self.read_and_cache(&mut data_cache, &mut timeline, frame, &path)?,
        };

        match timeline.earliest_error(frame) {
            Some((_, error)) => Err(error.clone()),
            None => Ok(value),
        }
    }

    fn read_and_cache(
        &self,
        data_cache: &mut DataCache,
        timeline: &mut GameTimeline<Arc<DllGameMemory>, Controller>,
        frame: u32,
        path: &Arc<GlobalDataPath>,
    ) -> Result<Value, Error> {
        let slot = timeline.frame(frame, false).0;
        let slot_memory = self.memory.with_slot(slot);

        data_cache.preload_frame(frame, &slot_memory);

        let value = path.read(&slot_memory)?;
        data_cache.insert(frame, path, value.clone());

        Ok(value)
    }

    /// Write a value on the given frame.
    ///
    /// See the crate documentation for [wafel_data_path] for the path syntax.
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
    /// See the crate documentation for [wafel_data_path] for the path syntax.
    ///
    /// Returns an error if the data path fails to compile.
    /// Note that this method does not return an error if there is a write error, e.g. null
    /// pointer dereference.
    /// Instead, write errors will be returned if `read` is called on a frame later than or
    /// equal to `frame`.
    pub fn try_write(&mut self, frame: u32, path: &str, value: Value) -> Result<(), Error> {
        let path = self.data_path(path)?;
        self.with_controller_mut(|controller| controller.write(frame, &path, value));
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
            Err(error) => panic!("Error:\n  failed to reset '{}':\n{}\n", path, error),
        }
    }

    /// Clear all previous calls to [write](Self::write) with the given `frame` and `path`.
    ///
    /// The `path` string must match the one passed to `write` exactly. If it is different,
    /// then this method will do nothing.
    ///
    /// Returns an error if the data path fails to compile.
    pub fn try_reset(&mut self, frame: u32, path: &str) -> Result<(), Error> {
        let path = self.data_path(path)?;
        self.with_controller_mut(|controller| controller.reset(frame, &path));
        Ok(())
    }

    fn with_controller_mut(&mut self, func: impl FnOnce(&mut Controller) -> InvalidatedFrames) {
        let timeline = self.timeline.get_mut().unwrap();
        let data_cache = self.data_cache.get_mut().unwrap();

        timeline.with_controller_mut(|controller| {
            let invalidated_frames = func(controller);
            if let InvalidatedFrames::StartingAt(frame) = invalidated_frames {
                data_cache.invalidate_frame(frame);
            }
            invalidated_frames
        });
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

#[derive(Debug, Clone, Default)]
struct Controller {
    edits: HashMap<u32, Vec<(Arc<GlobalDataPath>, Value)>>,
}

impl Controller {
    fn new() -> Self {
        Self::default()
    }

    fn write(
        &mut self,
        frame: u32,
        data_path: &Arc<GlobalDataPath>,
        value: Value,
    ) -> InvalidatedFrames {
        let frame_edits = self.edits.entry(frame).or_default();
        frame_edits.retain(|(edit_path, _)| !Arc::ptr_eq(data_path, edit_path));
        frame_edits.push((Arc::clone(data_path), value));
        InvalidatedFrames::StartingAt(frame)
    }

    fn reset(&mut self, frame: u32, data_path: &Arc<GlobalDataPath>) -> InvalidatedFrames {
        if let Some(frame_edits) = self.edits.get_mut(&frame) {
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
}

impl<M: GameMemory> GameController<M> for Controller {
    type Error = Error;

    fn apply(&self, memory: &M, slot: &mut M::Slot, frame: u32) -> Vec<Self::Error> {
        let mut errors = Vec::new();
        if let Some(frame_edits) = self.edits.get(&frame) {
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
