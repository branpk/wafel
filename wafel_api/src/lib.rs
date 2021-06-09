// #![warn(
//     missing_docs,
//     missing_debug_implementations,
//     rust_2018_idioms,
//     unreachable_pub
// )]

mod data_cache;

use std::{
    collections::HashMap,
    error, fmt,
    sync::{Arc, Mutex},
};

use data_cache::DataCache;
use wafel_data_path::{DataPathError, GlobalDataPath};
pub use wafel_data_type::Value;
use wafel_layout::{DataLayout, DllLayout, DllLayoutError, SM64ExtrasError};
use wafel_memory::{DllGameMemory, DllLoadError, GameMemory, MemoryError};
use wafel_timeline::{GameController, GameTimeline, InvalidatedFrames};

// TODO: Data cache

pub struct Timeline {
    memory: Arc<DllGameMemory>,
    layout: Arc<DataLayout>,
    timeline: Mutex<GameTimeline<Arc<DllGameMemory>, Controller>>,
    data_path_cache: Mutex<HashMap<String, Arc<GlobalDataPath>>>,
    data_cache: Mutex<DataCache>,
}

impl Timeline {
    #[track_caller]
    pub unsafe fn open(dll_path: &str) -> Self {
        match Self::try_open(dll_path) {
            Ok(timeline) => timeline,
            Err(error) => panic!("Error:\n  failed to open {}:\n  {}\n", dll_path, error),
        }
    }

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

    #[track_caller]
    pub fn read(&self, frame: u32, path: &str) -> Value {
        match self.try_read(frame, path) {
            Ok(value) => value,
            Err(error) => panic!("Error:\n  failed to read '{}':\n  {}\n", path, error),
        }
    }

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

    // TODO: In docs for write, mention overlapping writes with different paths

    #[track_caller]
    pub fn write(&mut self, frame: u32, path: &str, value: Value) {
        match self.try_write(frame, path, value) {
            Ok(()) => {}
            Err(error) => panic!("Error:\n  failed to write '{}':\n{}\n", path, error),
        }
    }

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
}

#[derive(Debug, Clone)]
pub enum Error {
    DllLayoutError(DllLayoutError),
    SM64ExtrasError(SM64ExtrasError),
    DllLoadError(DllLoadError),
    DataPathError(DataPathError),
    MemoryError(MemoryError),
    ApplyEditError {
        path: Arc<GlobalDataPath>,
        value: Value,
        error: MemoryError,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::DllLayoutError(error) => write!(f, "{}", error),
            Error::SM64ExtrasError(error) => write!(f, "{}", error),
            Error::DllLoadError(error) => write!(f, "{}", error),
            Error::DataPathError(error) => write!(f, "{}", error),
            Error::MemoryError(error) => write!(f, "{}", error),
            Error::ApplyEditError { path, value, error } => {
                write!(f, "while applying edit {} = {}:\n  {}", path, value, error)
            }
        }
    }
}

impl error::Error for Error {}

impl From<DllLayoutError> for Error {
    fn from(v: DllLayoutError) -> Self {
        Self::DllLayoutError(v)
    }
}

impl From<SM64ExtrasError> for Error {
    fn from(v: SM64ExtrasError) -> Self {
        Self::SM64ExtrasError(v)
    }
}

impl From<DllLoadError> for Error {
    fn from(v: DllLoadError) -> Self {
        Self::DllLoadError(v)
    }
}

impl From<DataPathError> for Error {
    fn from(v: DataPathError) -> Self {
        Self::DataPathError(v)
    }
}

impl From<MemoryError> for Error {
    fn from(v: MemoryError) -> Self {
        Self::MemoryError(v)
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
