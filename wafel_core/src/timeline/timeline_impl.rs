use super::{data_cache::DataCache, slot_manager::SlotManager, SlotState, SlotStateMut, State};
use crate::{
    data_path::GlobalDataPath,
    error::Error,
    memory::{Address, Memory, Value},
};
use std::{cell::RefCell, time::Duration};

/// Applies edits at the end of each frame to control the simulation.
pub trait Controller<M: Memory> {
    /// Apply edits to the given state.
    fn apply(&self, state: &mut impl SlotStateMut<Memory = M>) -> Result<(), Error>;
}

/// An abstraction allowing random access to any frame of the simulation.
#[derive(Debug)]
pub struct Timeline<M: Memory, C: Controller<M>> {
    slot_manager: SlotManager<M, C>,
    data_cache: RefCell<DataCache>,
}

impl<M: Memory, C: Controller<M>> Timeline<M, C> {
    /// Construct a new Timeline.
    ///
    /// Typically `memory` should be a freshly created `Memory` object.
    /// Otherwise, frame 0 will be defined as whatever the current contents of the
    /// base slot are.
    pub fn new(
        memory: M,
        base_slot: M::Slot,
        controller: C,
        num_backup_slots: usize,
    ) -> Result<Self, Error> {
        Ok(Self {
            slot_manager: SlotManager::new(memory, base_slot, controller, num_backup_slots)?,
            data_cache: RefCell::new(DataCache::new()),
        })
    }

    /// Destruct into the memory, base slot, and controller.
    ///
    /// The base slot is restored to the power-on state.
    pub fn into_parts(self) -> Result<(M, M::Slot, C), Error> {
        self.slot_manager.into_parts()
    }

    /// Get the memory that backs this timeline.
    pub fn memory(&self) -> &M {
        self.slot_manager.memory()
    }

    /// Get the controller.
    pub fn controller(&self) -> &C {
        self.slot_manager.controller()
    }

    /// Get a mutable reference to the controller.
    pub fn with_controller_mut(&mut self, func: impl FnOnce(&mut C) -> InvalidatedFrames) {
        let invalidated_frames = func(self.slot_manager.controller_mut());
        if let InvalidatedFrames::StartingAt(frame) = invalidated_frames {
            self.slot_manager.invalidate_frame(frame);
            self.data_cache.borrow_mut().invalidate_frame(frame);
        }
    }

    /// Get the state for a given frame.
    ///
    /// This method bypasses the data cache.
    ///
    /// Generally, only one state should be kept alive at a time. Accessing one of the states
    /// may result in a panic.
    pub fn frame_uncached(&self, frame: u32) -> Result<impl SlotState<Memory = M> + '_, Error> {
        self.slot_manager.frame(frame)
    }

    /// Get the state for a given frame.
    ///
    /// This method uses the data cache when accessing data.
    ///
    /// Generally, only one state should be kept alive at a time. Accessing one of the states
    /// may result in a panic.
    pub fn frame(&self, frame: u32) -> Result<impl State<Memory = M> + '_, Error> {
        Ok(TimelineState {
            timeline: self,
            frame,
        })
    }

    fn path_read_cached(&self, frame: u32, path: &GlobalDataPath) -> Result<Value, Error> {
        let cached_value = self.data_cache.borrow_mut().get(frame, path);
        match cached_value {
            Some(value) => Ok(value),
            None => {
                let state = self.frame_uncached(frame)?;
                let mut data_cache = self.data_cache.borrow_mut();

                data_cache.preload_frame(&state);

                let value = state.path_read(path)?;
                data_cache.insert(frame, path, value.clone());

                Ok(value)
            }
        }
    }

    /// Get an immutable view of the base slot.
    ///
    /// This can be used for running internal functions in the base slot if they have no
    /// potential side effects.
    ///
    /// # Panics
    ///
    /// Panics if another slot is requested while this one is still held.
    pub fn base_slot(&self, frame: u32) -> Result<impl SlotState<Memory = M> + '_, Error> {
        self.slot_manager.base_slot(frame)
    }

    /// Get a mutable view of the base slot.
    ///
    /// This can be used for running internal functions in the base slot that may have a side
    /// effect.
    pub fn base_slot_mut(
        &mut self,
        frame: u32,
    ) -> Result<impl SlotStateMut<Memory = M> + '_, Error> {
        self.slot_manager.base_slot_mut(frame)
    }

    /// Set a hotspot with a given name.
    ///
    /// A hotspot is a hint to the algorithm that scrolling should be smooth near the
    /// given frame.
    ///
    /// Note that the optimal `num_backup_slots` is proportional to the number
    /// of hotspots.
    pub fn set_hotspot(&mut self, name: &str, frame: u32) {
        self.slot_manager.set_hotspot(name, frame);
    }

    /// Delete a hotspot with the given name, if it exists.
    pub fn delete_hotspot(&mut self, name: &str) {
        self.slot_manager.delete_hotspot(name);
    }

    /// Perform housekeeping to improve scrolling near hotspots.
    pub fn balance_distribution(&mut self, max_run_time: Duration) -> Result<(), Error> {
        self.slot_manager.balance_distribution(max_run_time)
    }

    /// Return the set of currently loaded frames for debugging purposes.
    pub fn cached_frames(&self) -> Vec<u32> {
        self.slot_manager.cached_frames()
    }

    /// Return the number of frame advances since the timeline was created.
    pub fn num_advances(&self) -> usize {
        self.slot_manager.num_advances()
    }

    /// Return the number of slot copies since the timeline was created.
    pub fn num_copies(&self) -> usize {
        self.slot_manager.num_copies()
    }

    /// Return the size of the data cache in bytes.
    pub fn data_size_cache(&self) -> usize {
        self.data_cache.borrow().byte_size()
    }
}

/// A set of frames that should be invalidated after a controller mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub enum InvalidatedFrames {
    /// Invalidate states at and after the given frame.
    StartingAt(u32),
    /// No frames need to be invalidated.
    None,
}

impl InvalidatedFrames {
    /// Set `self` to None.
    pub fn clear(&mut self) {
        *self = InvalidatedFrames::None;
    }

    /// Include `frame` in the set.
    pub fn include(&mut self, frame: u32) {
        match self {
            Self::StartingAt(prev_frame) => *prev_frame = frame.min(*prev_frame),
            Self::None => *self = Self::StartingAt(frame),
        }
    }

    /// The union of two sets of frames.
    pub fn union(mut self, other: Self) -> Self {
        if let Self::StartingAt(frame) = other {
            self.include(frame);
        }
        self
    }
}

#[derive(Debug)]
struct TimelineState<'a, M: Memory, C: Controller<M>> {
    timeline: &'a Timeline<M, C>,
    frame: u32,
}

impl<'a, M: Memory, C: Controller<M>> State for TimelineState<'a, M, C> {
    type Memory = M;

    fn memory(&self) -> &Self::Memory {
        self.timeline.memory()
    }

    fn frame(&self) -> u32 {
        self.frame
    }

    fn path_address(&self, path: &GlobalDataPath) -> Result<Option<Address>, Error> {
        // Uncached for now (could also skip frame request in common case)
        self.timeline.frame_uncached(self.frame)?.path_address(path)
    }

    fn path_read(&self, path: &GlobalDataPath) -> Result<Value, Error> {
        self.timeline.path_read_cached(self.frame, path)
    }
}
