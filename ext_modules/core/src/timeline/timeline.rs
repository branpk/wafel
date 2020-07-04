use super::{slot_manager::SlotManager, SlotState, SlotStateMut, State};
use crate::{error::Error, memory::Memory};
use std::{collections::HashSet, time::Duration};

/// Applies edits at the end of each frame to control the simulation.
pub trait Controller<M: Memory> {
    /// Apply edits to the given state.
    fn apply(&self, state: &mut impl SlotStateMut<Memory = M>) -> Result<(), Error>;
}

/// An abstraction allowing random access to any frame of the simulation.
#[derive(Debug)]
pub struct Timeline<M: Memory, C: Controller<M>> {
    slot_manager: SlotManager<M, C>,
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
        })
    }

    /// Destruct into the memory, base slot, and controller.
    ///
    /// The base slot is restored to the power-on state.
    pub fn into_parts(self) -> Result<(M, M::Slot, C), Error> {
        self.slot_manager.into_parts()
    }

    /// Get the controller.
    pub fn controller(&self) -> &C {
        self.slot_manager.controller()
    }

    /// Get a mutable reference to the controller.
    ///
    /// `invalidated_frame` should be the first frame whose state may change
    /// as a result of mutations to the controller.
    pub fn controller_mut(&mut self, invalidated_frame: u32) -> &mut C {
        self.slot_manager.controller_mut(invalidated_frame)
    }

    /// Get a slot containing the state for a given frame.
    ///
    /// This method takes `&self` to be semantically correct, but it currently
    /// doesn't support overlapping calls due to the way the internal caching works.
    ///
    /// # Panics
    ///
    /// Panics if a different `State` is still in scope when this method is called.
    pub fn frame<'a>(&'a self, frame: u32) -> Result<impl State<Memory = M> + 'a, Error> {
        self.slot_manager.frame(frame)
    }

    /// Get an immutable view of the base slot.
    ///
    /// This can be used for running internal functions in the base slot if they have no
    /// potential side effects.
    ///
    /// # Panics
    ///
    /// See the `frame` method documentation.
    pub fn base_slot<'a>(&'a self, frame: u32) -> Result<impl SlotState<Memory = M> + 'a, Error> {
        self.slot_manager.base_slot(frame)
    }

    /// Get a mutable view of the base slot.
    ///
    /// This can be used for running internal functions in the base slot that may have a side
    /// effect.
    pub fn base_slot_mut<'a>(
        &'a mut self,
        frame: u32,
    ) -> Result<impl SlotStateMut<Memory = M> + 'a, Error> {
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
    pub fn loaded_frames(&self) -> HashSet<u32> {
        self.slot_manager.loaded_frames()
    }
}
