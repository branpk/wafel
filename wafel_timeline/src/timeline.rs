use std::{
    collections::{BTreeMap, HashMap, HashSet},
    time::{Duration, Instant},
};

use rand::prelude::*;
use wafel_memory::GameMemory;

use crate::{
    slots::{Frame, SlotIndex, SlotWrapper, Slots},
    GameController, InvalidatedFrames,
};

// TODO: Async loading

#[derive(Debug)]
pub struct GameTimeline<M: GameMemory, C: GameController<M>> {
    memory: M,
    controller: C,
    slots: Slots<M>,
    hotspots: HashMap<String, u32>,
    errors: BTreeMap<u32, Option<C::Error>>,
}

impl<M: GameMemory, C: GameController<M>> GameTimeline<M, C> {
    /// Construct a new GameTimeline.
    ///
    /// `memory` should be a freshly created `Memory` object.
    /// Otherwise, frame 0 will be defined as whatever the current contents of the
    /// base slot are.
    pub fn new(mut memory: M, base_slot: M::Slot, controller: C, num_backup_slots: usize) -> Self {
        let slots = Slots::new(&mut memory, base_slot, num_backup_slots);
        Self {
            memory,
            controller,
            slots,
            hotspots: HashMap::new(),
            errors: BTreeMap::new(),
        }
    }

    /// Destruct into the memory, base slot, and controller.
    ///
    /// The base slot is restored to the power-on state.
    pub fn into_parts(self) -> (M, M::Slot, C) {
        let mut base_slot = self.slots.base.slot;
        self.memory
            .copy_slot(&mut base_slot, &self.slots.power_on.slot);
        (self.memory, base_slot, self.controller)
    }

    fn request_frame(&mut self, requested_frame: u32, require_base: bool) -> SlotIndex {
        // Function to compute the number of copies and updates that would be required to reach
        // the requested frame from a given slot
        let work_from = |slot: &SlotWrapper<M::Slot>| -> (u32, u32) {
            let slot_frame = match slot.frame {
                Frame::At(frame) => frame,
                Frame::PowerOn => 0,
                Frame::Unknown => unimplemented!(),
            };
            if slot_frame == requested_frame {
                return (0, 0);
            }
            let copies = if slot.is_base { 0 } else { 1 };
            assert!(slot_frame <= requested_frame);
            let updates = requested_frame - slot_frame;
            (copies, updates)
        };

        // Computes an approximate time cost of updating a slot to the requested frame
        let cost_from = |slot: &SlotWrapper<M::Slot>| -> u32 {
            let (copies, updates) = work_from(slot);
            10 * copies + updates
        };

        // Find the slot with the lowest cost
        let nearest_slot: &SlotWrapper<M::Slot> = self
            .slots
            .iter()
            .filter(|slot| match slot.frame {
                Frame::At(frame) => frame <= requested_frame,
                Frame::PowerOn => true,
                Frame::Unknown => false,
            })
            .min_by_key(|slot| cost_from(slot))
            .unwrap(); // power_on_slot is always included

        // Fast path (avoids a copy when nearest_slot is not the base slot)
        let use_nearest_slot = nearest_slot.frame == Frame::At(requested_frame)
            && (!require_base || nearest_slot.is_base);

        let result_slot = if use_nearest_slot {
            nearest_slot
        } else {
            // Copy to base slot
            let nearest_slot_index = nearest_slot.index;
            self.slots
                .copy_slot(&self.memory, SlotIndex::Base, nearest_slot_index);

            // Advance base slot to requested frame
            while self.slots.base.frame != Frame::At(requested_frame) {
                let new_frame = self.slots.advance_base_slot(&self.memory);
                let error = self
                    .controller
                    .apply(&self.memory, &mut self.slots.base.slot, new_frame)
                    .err();
                self.errors.insert(new_frame, error);
            }
            &self.slots.base
        };

        assert!(result_slot.frame == Frame::At(requested_frame));
        assert!(!require_base || result_slot.is_base);

        result_slot.index
    }

    /// Returns a slot holding the state for the given frame, and the error that the
    /// controller returned on that frame, if any.
    pub fn frame(&mut self, frame: u32, require_base: bool) -> (&M::Slot, &Option<C::Error>) {
        let slot_index = self.request_frame(frame, require_base);
        let slot = &self.slots.get(slot_index).slot;
        let error = self
            .errors
            .get(&frame)
            .expect("missing error for requested frame");
        (slot, error)
    }

    /// Returns a mutable slot holding the state for the given frame, and the error that the
    /// controller returned on that frame, if any.
    ///
    /// Note that mutating the slot has no effect on the timeline, even on the requested
    /// frame.
    /// This method is primarily useful for running functions on the base slot without worrying
    /// about the function mutating data.
    pub fn frame_mut(
        &mut self,
        frame: u32,
        require_base: bool,
    ) -> (&mut M::Slot, &Option<C::Error>) {
        let slot_index = self.request_frame(frame, require_base);
        let slot_wrapper = self.slots.get_mut(slot_index);

        // Invalidate the state so that the slot isn't used for future requests
        slot_wrapper.frame = Frame::Unknown;

        let slot = &mut slot_wrapper.slot;
        let error = self
            .errors
            .get(&frame)
            .expect("missing error for requested frame");
        (slot, error)
    }

    /// Perform housekeeping to keep the hotspots fast to scroll near.
    pub fn balance_distribution(&mut self, max_run_time: Duration) {
        let start_time = Instant::now();

        let alignments = vec![1, 15, 40, 145, 410, 1505, 4010, 14005];
        let mut target_frames: Vec<u32> = Vec::new();
        for &hotspot in self.hotspots.values() {
            for &alignment in &alignments {
                target_frames.push(hotspot - (hotspot % alignment));
            }
        }
        target_frames.sort_unstable();
        target_frames.dedup();

        let mut used_slots: HashSet<SlotIndex> = HashSet::new();
        for target_frame in target_frames {
            if start_time.elapsed() > max_run_time {
                break;
            }

            let matching_slot: Option<&SlotWrapper<M::Slot>> = self
                .slots
                .iter()
                .find(|slot| !slot.is_base && slot.frame == Frame::At(target_frame));
            if let Some(matching_slot) = matching_slot {
                used_slots.insert(matching_slot.index);
                continue;
            }

            let source_slot = self.request_frame(target_frame, false);
            let available_slots: Vec<SlotIndex> = self
                .slots
                .iter_mut()
                .filter(|slot| {
                    !slot.is_base && slot.index != source_slot && !used_slots.contains(&slot.index)
                })
                .map(|slot| slot.index)
                .collect();
            let dest_slot = available_slots.choose(&mut rand::thread_rng()).cloned();

            match dest_slot {
                Some(dest_slot) => self.slots.copy_slot(&self.memory, dest_slot, source_slot),
                None => eprintln!("Using suboptimal number of slots"), // TODO: Logger
            }
            // TODO: Add dest_slot to used_slots?
        }
    }

    pub fn memory(&self) -> &M {
        &self.memory
    }

    pub fn controller(&self) -> &C {
        &self.controller
    }

    pub fn with_controller_mut(&mut self, func: impl FnOnce(&mut C) -> InvalidatedFrames) {
        let invalidated_frames = func(&mut self.controller);
        if let InvalidatedFrames::StartingAt(frame) = invalidated_frames {
            self.invalidate_frame(frame);
        }
    }

    pub fn invalidate_frame(&mut self, invalidated_frame: u32) {
        for slot in self.slots.iter_mut() {
            if let Frame::At(slot_frame) = slot.frame {
                if slot_frame >= invalidated_frame {
                    slot.frame = Frame::Unknown;
                }
            }
        }
        self.errors.split_off(&invalidated_frame);
    }

    pub fn set_hotspot(&mut self, name: &str, frame: u32) {
        self.hotspots.insert(name.to_owned(), frame);
    }

    pub fn delete_hotspot(&mut self, name: &str) {
        self.hotspots.remove(name);
    }

    pub fn cached_frames(&self) -> Vec<u32> {
        self.slots
            .iter()
            .filter_map(|slot| match slot.frame {
                Frame::At(frame) => Some(frame),
                Frame::PowerOn => Some(0),
                Frame::Unknown => None,
            })
            .collect()
    }

    pub fn num_advances(&self) -> usize {
        self.slots.num_advances
    }

    pub fn num_copies(&self) -> usize {
        self.slots.num_copies
    }
}
