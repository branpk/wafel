//! Implementation of timeline algorithm.

use super::{slot_state_impl::SlotStateImpl, Controller, SlotState, SlotStateMut, State};
use crate::{error::Error, memory::Memory};
use itertools::{iproduct, Itertools};
use rand::seq::SliceRandom;
use std::{
    cell::{RefCell, RefMut},
    collections::{HashMap, HashSet},
    iter,
    time::{Duration, Instant},
};

/// A slot and information about its current content.
#[derive(Debug)]
struct SlotWrapper<S> {
    index: SlotIndex,
    slot: S,
    is_base: bool,
    frame: Frame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Frame {
    /// The slot holds the data for a specific frame.
    At(u32),
    /// The slot holds the power-on state.
    ///
    /// This differs from At(0) in that At(0) includes the Controller edits for frame 0,
    /// while PowerOn doesn't include any edits.
    PowerOn,
    /// The slot's contents are unknown or invalid.
    Unknown,
}

/// Container to keep track of allocated slots and their contents.
#[derive(Debug)]
struct Slots<M: Memory> {
    /// Slot kept at the power-on state so that there is always a slot to fall back to.
    power_on: SlotWrapper<M::Slot>,
    base: SlotWrapper<M::Slot>,
    backups: Vec<SlotWrapper<M::Slot>>,
    /// Debug stat counting number of frame advances.
    num_advances: usize,
    /// Debug stat counting number of slot copies.
    num_copies: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SlotIndex {
    PowerOn,
    Base,
    Backup(usize),
}

impl<M: Memory> Slots<M> {
    fn get(&self, index: SlotIndex) -> &SlotWrapper<M::Slot> {
        match index {
            SlotIndex::PowerOn => &self.power_on,
            SlotIndex::Base => &self.base,
            SlotIndex::Backup(index) => &self.backups[index],
        }
    }

    fn get_mut(&mut self, index: SlotIndex) -> &mut SlotWrapper<M::Slot> {
        match index {
            SlotIndex::PowerOn => &mut self.power_on,
            SlotIndex::Base => &mut self.base,
            SlotIndex::Backup(index) => &mut self.backups[index],
        }
    }

    /// Return an iterator over all slots, including the power-on and base slots.
    fn iter(&self) -> impl Iterator<Item = &SlotWrapper<M::Slot>> {
        iter::once(&self.base)
            .chain(self.backups.iter())
            .chain(iter::once(&self.power_on))
    }

    /// Return an iterator over all mutable slots, i.e. excluding the power-on slot.
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut SlotWrapper<M::Slot>> {
        iter::once(&mut self.base).chain(self.backups.iter_mut())
    }
}

fn copy_slot<M: Memory>(
    memory: &M,
    slots: &mut Slots<M>,
    dst_index: SlotIndex,
    src_index: SlotIndex,
) -> Result<(), Error> {
    if dst_index != src_index {
        let (dst, src) = unsafe {
            let src = slots.get(src_index) as *const _;
            let dst = slots.get_mut(dst_index);
            let src: &SlotWrapper<_> = &*src;
            (dst, src)
        };

        memory.copy_slot(&mut dst.slot, &src.slot)?;
        dst.frame = src.frame;
        slots.num_copies = slots.num_copies.wrapping_add(1);
    }
    Ok(())
}

/// Advance the base slot's frame and apply controller edits.
///
/// The base slot's frame must not equal Frame::Unknown.
fn advance_frame<M: Memory, C: Controller<M>>(
    memory: &M,
    controller: &C,
    slots: &mut Slots<M>,
) -> Result<(), Error> {
    let base = &mut slots.base;

    let new_frame;
    match base.frame {
        Frame::PowerOn => {
            new_frame = 0;
        }
        Frame::At(frame) => {
            memory.advance_base_slot(&mut base.slot)?;
            new_frame = frame + 1;
            slots.num_advances = slots.num_advances.wrapping_add(1);
        }
        _ => panic!("base.frame = Frame::Unknown"),
    };

    base.frame = Frame::At(new_frame);
    controller.apply(&mut SlotStateImpl {
        memory,
        frame: new_frame,
        slot: &mut base.slot,
    })?;

    Ok(())
}

fn request_frame<M: Memory, C: Controller<M>>(
    memory: &M,
    controller: &C,
    slots: &mut Slots<M>,
    requested_frame: u32,
    require_base: bool,
) -> Result<SlotIndex, Error> {
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
        copy_slot(memory, slots, SlotIndex::Base, nearest_slot_index)?;

        // Advance base slot to requested frame
        while slots.base.frame != Frame::At(requested_frame) {
            advance_frame(memory, controller, slots)?;
        }
        &slots.base
    };

    Ok(result_slot.index)
}

#[derive(Debug)]
pub struct SlotManager<M: Memory, C: Controller<M>> {
    memory: M,
    controller: C,
    /// The slots that are owned by this manager.
    ///
    /// Since conceptually these are mostly used a cache of the state on various frames,
    /// we use interior mutability.
    slots: RefCell<Slots<M>>,
    hotspots: HashMap<String, u32>,
}

impl<M: Memory, C: Controller<M>> SlotManager<M, C> {
    pub fn new(
        memory: M,
        base_slot: M::Slot,
        controller: C,
        num_backup_slots: usize,
    ) -> Result<Self, Error> {
        let base_slot = SlotWrapper {
            index: SlotIndex::Base,
            slot: base_slot,
            is_base: true,
            frame: Frame::PowerOn,
        };

        let mut power_on_slot = SlotWrapper {
            index: SlotIndex::PowerOn,
            slot: memory.create_backup_slot()?,
            is_base: false,
            frame: Frame::PowerOn,
        };
        memory.copy_slot(&mut power_on_slot.slot, &base_slot.slot)?;

        let backup_slots: Vec<_> = iter::repeat_with(|| memory.create_backup_slot())
            .take(num_backup_slots)
            .collect::<Result<Vec<M::Slot>, Error>>()?
            .into_iter()
            .enumerate()
            .map(|(index, slot)| SlotWrapper {
                index: SlotIndex::Backup(index),
                slot,
                is_base: false,
                frame: Frame::Unknown,
            })
            .collect();

        Ok(Self {
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
        })
    }

    /// Destruct into the memory, base slot, and controller.
    ///
    /// The base slot is restored to the power-on state.
    pub fn into_parts(self) -> Result<(M, M::Slot, C), Error> {
        let slots = self.slots.into_inner();
        let mut base_slot = slots.base.slot;
        self.memory
            .copy_slot(&mut base_slot, &slots.power_on.slot)?;
        Ok((self.memory, base_slot, self.controller))
    }

    fn borrow_slot_state<'a>(
        &'a self,
        frame: u32,
        require_base: bool,
    ) -> Result<impl SlotState<Memory = M> + 'a, Error> {
        let mut slots = self
            .slots
            .try_borrow_mut()
            .expect("only one state can be requested at a time");

        let slot_index = request_frame(
            &self.memory,
            &self.controller,
            &mut slots,
            frame,
            require_base,
        )?;
        let slot = RefMut::map(slots, |slots| {
            let slot_wrapper = slots.get_mut(slot_index);
            assert!(slot_wrapper.frame == Frame::At(frame));
            assert!(!require_base || slot_wrapper.is_base);
            &mut slot_wrapper.slot
        });

        Ok(SlotStateImpl {
            memory: &self.memory,
            frame,
            slot,
        })
    }

    fn slot_state_mut<'a>(
        &'a mut self,
        frame: u32,
        require_base: bool,
    ) -> Result<impl SlotStateMut<Memory = M> + 'a, Error> {
        let slots = self.slots.get_mut();

        let slot_index = request_frame(&self.memory, &self.controller, slots, frame, require_base)?;

        let slot_wrapper = slots.get_mut(slot_index);
        assert!(slot_wrapper.frame == Frame::At(frame));
        assert!(!require_base || slot_wrapper.is_base);

        // Invalidate the state so that the slot isn't used for future requests
        slot_wrapper.frame = Frame::Unknown;

        Ok(SlotStateImpl {
            memory: &self.memory,
            frame,
            slot: &mut slot_wrapper.slot,
        })
    }

    pub fn frame<'a>(&'a self, frame: u32) -> Result<impl State<Memory = M> + 'a, Error> {
        self.borrow_slot_state(frame, false)
    }

    pub fn base_slot<'a>(&'a self, frame: u32) -> Result<impl SlotState<Memory = M> + 'a, Error> {
        self.borrow_slot_state(frame, true)
    }

    pub fn base_slot_mut<'a>(
        &'a mut self,
        frame: u32,
    ) -> Result<impl SlotStateMut<Memory = M> + 'a, Error> {
        self.slot_state_mut(frame, true)
    }

    /// Perform housekeeping to keep the hotspots fast to scroll near.
    pub fn balance_distribution(&mut self, max_run_time: Duration) -> Result<(), Error> {
        let slots = self.slots.get_mut();

        let start_time = Instant::now();

        let alignments = vec![1, 15, 40, 145, 410, 1505, 4010, 14005];
        let target_frames: Vec<u32> = iproduct!(self.hotspots.values(), alignments.iter())
            .map(|(hotspot, alignment)| hotspot - (hotspot % alignment))
            .sorted()
            .dedup()
            .collect();

        let mut used_slots: HashSet<SlotIndex> = HashSet::new();
        for target_frame in target_frames {
            if start_time.elapsed() > max_run_time {
                break;
            }

            let matching_slot: Option<&SlotWrapper<M::Slot>> = slots
                .iter()
                .filter(|slot| !slot.is_base && slot.frame == Frame::At(target_frame))
                .next();
            if let Some(matching_slot) = matching_slot {
                used_slots.insert(matching_slot.index);
                continue;
            }

            let source_slot =
                request_frame(&self.memory, &self.controller, slots, target_frame, false)?;
            let available_slots: Vec<SlotIndex> = slots
                .iter_mut()
                .filter(|slot| {
                    !slot.is_base && slot.index != source_slot && !used_slots.contains(&slot.index)
                })
                .map(|slot| slot.index)
                .collect();
            let dest_slot = available_slots.choose(&mut rand::thread_rng()).cloned();

            match dest_slot {
                Some(dest_slot) => copy_slot(&self.memory, slots, dest_slot, source_slot)?,
                None => eprintln!("Using suboptimal number of slots"), // TODO: Logger
            }
            // TODO: Add dest_slot to used_slots?
        }

        Ok(())
    }

    pub fn memory(&self) -> &M {
        &self.memory
    }

    pub fn controller(&self) -> &C {
        &self.controller
    }

    pub fn controller_mut(&mut self) -> &mut C {
        &mut self.controller
    }

    pub fn invalidate_frame(&mut self, invalidated_frame: u32) {
        for slot in self.slots.get_mut().iter_mut() {
            if let Frame::At(slot_frame) = slot.frame {
                if slot_frame >= invalidated_frame {
                    slot.frame = Frame::Unknown;
                }
            }
        }
    }

    pub fn set_hotspot(&mut self, name: &str, frame: u32) {
        self.hotspots.insert(name.to_owned(), frame);
    }

    pub fn delete_hotspot(&mut self, name: &str) {
        self.hotspots.remove(name);
    }

    pub fn cached_frames(&self) -> Vec<u32> {
        self.slots
            .borrow()
            .iter()
            .filter_map(|slot| match slot.frame {
                Frame::At(frame) => Some(frame),
                Frame::PowerOn => Some(0),
                Frame::Unknown => None,
            })
            .collect()
    }

    pub fn num_advances(&self) -> usize {
        self.slots.borrow().num_advances
    }

    pub fn num_copies(&self) -> usize {
        self.slots.borrow().num_copies
    }
}
