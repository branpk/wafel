use std::{
    cell::{RefCell, RefMut},
    collections::HashMap,
    iter,
};

use wafel_memory::{GameMemory, MemoryError};

use crate::slots::{Frame, SlotIndex, SlotWrapper, Slots};

// TODO: Whenever controller is modified, scan through TAS to validate it and
//       pre-emptively save slots.
//       (Need to keep track of end frame, but probably should anyway).
//       This ensures low latency when requesting a frame, so async loading logic can be
//       implemented around controller_mut() instead of frame()
//       It also allows us to panic instead of returning an error when requesting a frame.
//       I.e. fn new(...) -> Result<Self, C::Error>
//            fn frame(...) -> State
//            fn with_controller_mut(...) -> Result<(), C::Error>

/// Applies edits at the end of each frame to control the simulation.
pub trait GameController<M: GameMemory> {
    type Error;

    /// Apply edits to the given state.
    fn apply(&self, memory: &M, slot: &mut M::Slot, frame: u32) -> Result<(), Self::Error>;
}

fn request_frame<M: GameMemory, C: GameController<M>>(
    memory: &M,
    controller: &C,
    slots: &mut Slots<M>,
    requested_frame: u32,
    require_base: bool,
) -> Result<SlotIndex, C::Error> {
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
    let nearest_slot: &SlotWrapper<M::Slot> = slots
        .iter()
        .filter(|slot| match slot.frame {
            Frame::At(frame) => frame <= requested_frame,
            Frame::PowerOn => true,
            Frame::Unknown => false,
        })
        .min_by_key(|slot| cost_from(slot))
        .unwrap(); // power_on_slot is always included

    // Fast path (avoids a copy when nearest_slot is not the base slot)
    let use_nearest_slot =
        nearest_slot.frame == Frame::At(requested_frame) && (!require_base || nearest_slot.is_base);

    let result_slot = if use_nearest_slot {
        nearest_slot
    } else {
        // Copy to base slot
        let nearest_slot_index = nearest_slot.index;
        slots.copy_slot(memory, SlotIndex::Base, nearest_slot_index);

        // Advance base slot to requested frame
        while slots.base.frame != Frame::At(requested_frame) {
            let new_frame = slots.advance_base_slot(memory);
            controller.apply(memory, &mut slots.base.slot, new_frame)?;
        }
        &slots.base
    };

    Ok(result_slot.index)
}

#[derive(Debug)]
pub struct GameTimeline<M: GameMemory, C: GameController<M>> {
    memory: M,
    controller: C,
    /// The slots that are owned by this manager.
    ///
    /// Since these are mostly used as a cache of the state on various frames,
    /// we use interior mutability.
    slots: RefCell<Slots<M>>,
    hotspots: HashMap<String, u32>,
}

impl<M: GameMemory, C: GameController<M>> GameTimeline<M, C> {
    /// Construct a new GameTimeline.
    ///
    /// `memory` should be a freshly created `Memory` object.
    /// Otherwise, frame 0 will be defined as whatever the current contents of the
    /// base slot are.
    pub fn new(memory: M, base_slot: M::Slot, controller: C, num_backup_slots: usize) -> Self {
        let base_slot = SlotWrapper {
            index: SlotIndex::Base,
            slot: base_slot,
            is_base: true,
            frame: Frame::PowerOn,
        };

        let mut power_on_slot = SlotWrapper {
            index: SlotIndex::PowerOn,
            slot: memory.create_backup_slot(),
            is_base: false,
            frame: Frame::PowerOn,
        };
        memory.copy_slot(&mut power_on_slot.slot, &base_slot.slot);

        let backup_slots: Vec<_> = iter::repeat_with(|| memory.create_backup_slot())
            .take(num_backup_slots)
            .enumerate()
            .map(|(index, slot)| SlotWrapper {
                index: SlotIndex::Backup(index),
                slot,
                is_base: false,
                frame: Frame::Unknown,
            })
            .collect();

        Self {
            memory,
            controller,
            slots: RefCell::new(Slots {
                power_on: power_on_slot,
                base: base_slot,
                backups: backup_slots,
                num_advances: 0,
                num_copies: 0,
            }),
            hotspots: HashMap::new(),
        }
    }

    /// Destruct into the memory, base slot, and controller.
    ///
    /// The base slot is restored to the power-on state.
    pub fn into_parts(self) -> (M, M::Slot, C) {
        let slots = self.slots.into_inner();
        let mut base_slot = slots.base.slot;
        self.memory.copy_slot(&mut base_slot, &slots.power_on.slot);
        (self.memory, base_slot, self.controller)
    }

    //     fn borrow_slot_state(
    //         &'_ self,
    //         frame: u32,
    //         require_base: bool,
    //     ) -> Result<impl SlotState<Memory = M> + '_, Error> {
    //         let mut slots = self
    //             .slots
    //             .try_borrow_mut()
    //             .expect("only one state can be requested at a time");
    //
    //         let slot_index = request_frame(
    //             &self.memory,
    //             &self.controller,
    //             &mut slots,
    //             frame,
    //             require_base,
    //         )?;
    //         let slot = RefMut::map(slots, |slots| {
    //             let slot_wrapper = slots.get_mut(slot_index);
    //             assert!(slot_wrapper.frame == Frame::At(frame));
    //             assert!(!require_base || slot_wrapper.is_base);
    //             &mut slot_wrapper.slot
    //         });
    //
    //         Ok(SlotStateImpl {
    //             memory: &self.memory,
    //             frame,
    //             slot,
    //         })
    //     }

    //     fn slot_state_mut(
    //         &mut self,
    //         frame: u32,
    //         require_base: bool,
    //     ) -> Result<impl SlotStateMut<Memory = M> + '_, Error> {
    //         let slots = self.slots.get_mut();
    //
    //         let slot_index = request_frame(&self.memory, &self.controller, slots, frame, require_base)?;
    //
    //         let slot_wrapper = slots.get_mut(slot_index);
    //         assert!(slot_wrapper.frame == Frame::At(frame));
    //         assert!(!require_base || slot_wrapper.is_base);
    //
    //         // Invalidate the state so that the slot isn't used for future requests
    //         slot_wrapper.frame = Frame::Unknown;
    //
    //         Ok(SlotStateImpl {
    //             memory: &self.memory,
    //             frame,
    //             slot: &mut slot_wrapper.slot,
    //         })
    //     }
    //
    //     pub fn frame(&self, frame: u32) -> Result<impl SlotState<Memory = M> + '_, Error> {
    //         self.borrow_slot_state(frame, false)
    //     }
    //
    //     pub fn base_slot(&self, frame: u32) -> Result<impl SlotState<Memory = M> + '_, Error> {
    //         self.borrow_slot_state(frame, true)
    //     }
    //
    //     pub fn base_slot_mut(
    //         &mut self,
    //         frame: u32,
    //     ) -> Result<impl SlotStateMut<Memory = M> + '_, Error> {
    //         self.slot_state_mut(frame, true)
    //     }
    //
    //     /// Perform housekeeping to keep the hotspots fast to scroll near.
    //     pub fn balance_distribution(&mut self, max_run_time: Duration) -> Result<(), Error> {
    //         let slots = self.slots.get_mut();
    //
    //         let start_time = Instant::now();
    //
    //         let alignments = vec![1, 15, 40, 145, 410, 1505, 4010, 14005];
    //         let target_frames: Vec<u32> = iproduct!(self.hotspots.values(), alignments.iter())
    //             .map(|(hotspot, alignment)| hotspot - (hotspot % alignment))
    //             .sorted()
    //             .dedup()
    //             .collect();
    //
    //         let mut used_slots: HashSet<SlotIndex> = HashSet::new();
    //         for target_frame in target_frames {
    //             if start_time.elapsed() > max_run_time {
    //                 break;
    //             }
    //
    //             let matching_slot: Option<&SlotWrapper<M::Slot>> = slots
    //                 .iter()
    //                 .find(|slot| !slot.is_base && slot.frame == Frame::At(target_frame));
    //             if let Some(matching_slot) = matching_slot {
    //                 used_slots.insert(matching_slot.index);
    //                 continue;
    //             }
    //
    //             let source_slot =
    //                 request_frame(&self.memory, &self.controller, slots, target_frame, false)?;
    //             let available_slots: Vec<SlotIndex> = slots
    //                 .iter_mut()
    //                 .filter(|slot| {
    //                     !slot.is_base && slot.index != source_slot && !used_slots.contains(&slot.index)
    //                 })
    //                 .map(|slot| slot.index)
    //                 .collect();
    //             let dest_slot = available_slots.choose(&mut rand::thread_rng()).cloned();
    //
    //             match dest_slot {
    //                 Some(dest_slot) => copy_slot(&self.memory, slots, dest_slot, source_slot)?,
    //                 None => eprintln!("Using suboptimal number of slots"), // TODO: Logger
    //             }
    //             // TODO: Add dest_slot to used_slots?
    //         }
    //
    //         Ok(())
    //     }
    //
    //     pub fn memory(&self) -> &M {
    //         &self.memory
    //     }
    //
    //     pub fn controller(&self) -> &C {
    //         &self.controller
    //     }
    //
    //     pub fn controller_mut(&mut self) -> &mut C {
    //         &mut self.controller
    //     }
    //
    //     pub fn invalidate_frame(&mut self, invalidated_frame: u32) {
    //         for slot in self.slots.get_mut().iter_mut() {
    //             if let Frame::At(slot_frame) = slot.frame {
    //                 if slot_frame >= invalidated_frame {
    //                     slot.frame = Frame::Unknown;
    //                 }
    //             }
    //         }
    //     }
    //
    //     pub fn set_hotspot(&mut self, name: &str, frame: u32) {
    //         self.hotspots.insert(name.to_owned(), frame);
    //     }
    //
    //     pub fn delete_hotspot(&mut self, name: &str) {
    //         self.hotspots.remove(name);
    //     }
    //
    //     pub fn cached_frames(&self) -> Vec<u32> {
    //         self.slots
    //             .borrow()
    //             .iter()
    //             .filter_map(|slot| match slot.frame {
    //                 Frame::At(frame) => Some(frame),
    //                 Frame::PowerOn => Some(0),
    //                 Frame::Unknown => None,
    //             })
    //             .collect()
    //     }
    //
    //     pub fn num_advances(&self) -> usize {
    //         self.slots.borrow().num_advances
    //     }
    //
    //     pub fn num_copies(&self) -> usize {
    //         self.slots.borrow().num_copies
    //     }
}
