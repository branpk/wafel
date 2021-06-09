// #![warn(
//     missing_docs,
//     missing_debug_implementations,
//     rust_2018_idioms,
//     unreachable_pub
// )]

use std::{
    collections::HashMap,
    error, fmt,
    sync::{Arc, Mutex},
};

use wafel_data_path::{DataPathError, GlobalDataPath};
pub use wafel_data_type::Value;
use wafel_layout::{DataLayout, DllLayout, DllLayoutError};
use wafel_memory::{DllGameMemory, DllLoadError, GameMemory, MemoryError};
use wafel_timeline::{GameController, GameTimeline, InvalidatedFrames};

// TODO: Data cache

pub struct Timeline {
    memory: Arc<DllGameMemory>,
    layout: Arc<DataLayout>,
    timeline: Mutex<GameTimeline<Arc<DllGameMemory>, Controller>>,
    data_path_cache: Mutex<HashMap<String, Arc<GlobalDataPath>>>,
}

impl Timeline {
    #[track_caller]
    pub unsafe fn open(dll_path: &str) -> Self {
        Self::try_open(dll_path)
            .unwrap_or_else(|error| panic!("failed to open {}: {}", dll_path, error))
    }

    pub unsafe fn try_open(dll_path: &str) -> Result<Self, Error> {
        let mut layout = DllLayout::read(&dll_path)?.data_layout;
        layout.add_sm64_extras();
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
            data_path_cache: Default::default(),
        })
    }

    #[track_caller]
    pub fn read(&self, frame: u32, path: &str) -> Value {
        self.try_read(frame, path)
            .unwrap_or_else(|error| panic!("failed to read '{}': {}", path, error))
    }

    pub fn try_read(&self, frame: u32, path: &str) -> Result<Value, Error> {
        let path = self.data_path(path)?;
        let mut timeline = self.timeline.lock().unwrap();
        let (slot, error) = timeline.frame(frame, false);
        if let Some(error) = error {
            return Err(error.clone());
        }
        let value = path.read(&self.memory.with_slot(slot))?;
        Ok(value)
    }

    // Expose EditRange API:
    // - write_range(&mut self, frames: Range<u32>, path: &str, value: Value) -> EditRange
    // - write(&mut self, frames: Range<u32>, path: &str, value: Value) -> EditRange
    // -
    // - move_range(&mut self, id: EditRangeId, frames: Range<u32>)
    // - set_range_value(&mut self, id: EditRangeId, value: Value)
    // - delete_range(&mut self, id: EditRangeId)
    // etc
    // Then drag/drop functionality is implemented on top of this API
    //
    // OR: don't for now. Just use basic 1-frame write and implement the RangeEdits API on top
    // of that

    #[track_caller]
    pub fn write(&mut self, frame: u32, path: &str, value: Value) {
        self.try_write(frame, path, value)
            .unwrap_or_else(|error| panic!("failed to write '{}': {}", path, error))
    }

    pub fn try_write(&mut self, frame: u32, path: &str, value: Value) -> Result<(), Error> {
        let path = self.data_path(path)?;
        let timeline = self.timeline.get_mut().unwrap();
        timeline.with_controller_mut(|controller| controller.write(frame, &path, value));
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
}

fn check_sync<T: Send + Sync>() {}

fn check_all_sync() {
    check_sync::<Timeline>();
}

#[derive(Debug, Clone)]
pub enum Error {
    DllLayoutError(DllLayoutError),
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
            Error::DllLoadError(error) => write!(f, "{}", error),
            Error::DataPathError(error) => write!(f, "{}", error),
            Error::MemoryError(error) => write!(f, "{}", error),
            Error::ApplyEditError { path, value, error } => {
                write!(f, "while applying edit {} = {}: {}", path, value, error)
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
}

impl<M: GameMemory> GameController<M> for Controller {
    type Error = Error;

    fn apply(&self, memory: &M, slot: &mut M::Slot, frame: u32) -> Option<Self::Error> {
        let mut error = None;
        if let Some(frame_edits) = self.edits.get(&frame) {
            for (path, value) in frame_edits {
                path.write(&mut memory.with_slot_mut(slot), value.clone())
                    .unwrap_or_else(|e| {
                        error = Some(Error::ApplyEditError {
                            path: Arc::clone(path),
                            value: value.clone(),
                            error: e,
                        });
                    });
            }
        }
        error
    }
}
