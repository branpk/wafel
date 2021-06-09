use std::{fmt, iter};

use wafel_memory::GameMemory;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SlotIndex {
    PowerOn,
    Base,
    Backup(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Frame {
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

/// A slot and information about its current content.
#[derive(Debug)]
pub(crate) struct SlotWrapper<S> {
    pub(crate) index: SlotIndex,
    pub(crate) slot: S,
    pub(crate) is_base: bool,
    pub(crate) frame: Frame,
}

/// Container to keep track of allocated slots and their contents.
pub(crate) struct Slots<M: GameMemory> {
    /// Slot kept at the power-on state so that there is always a slot to fall back to.
    pub(crate) power_on: SlotWrapper<M::Slot>,
    pub(crate) base: SlotWrapper<M::Slot>,
    pub(crate) backups: Vec<SlotWrapper<M::Slot>>,
}

impl<M: GameMemory> Slots<M> {
    pub(crate) fn new(memory: &mut M, base_slot: M::Slot, num_backup_slots: usize) -> Self {
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
            power_on: power_on_slot,
            base: base_slot,
            backups: backup_slots,
        }
    }

    pub(crate) fn get(&self, index: SlotIndex) -> &SlotWrapper<M::Slot> {
        match index {
            SlotIndex::PowerOn => &self.power_on,
            SlotIndex::Base => &self.base,
            SlotIndex::Backup(index) => &self.backups[index],
        }
    }

    pub(crate) fn get_mut(&mut self, index: SlotIndex) -> &mut SlotWrapper<M::Slot> {
        match index {
            SlotIndex::PowerOn => &mut self.power_on,
            SlotIndex::Base => &mut self.base,
            SlotIndex::Backup(index) => &mut self.backups[index],
        }
    }

    /// Return an iterator over all slots, including the power-on and base slots.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &SlotWrapper<M::Slot>> {
        iter::once(&self.base)
            .chain(self.backups.iter())
            .chain(iter::once(&self.power_on))
    }

    /// Return an iterator over all mutable slots, i.e. excluding the power-on slot.
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut SlotWrapper<M::Slot>> {
        iter::once(&mut self.base).chain(self.backups.iter_mut())
    }

    pub(crate) fn copy_slot(&mut self, memory: &M, dst_index: SlotIndex, src_index: SlotIndex) {
        if dst_index != src_index {
            let (dst, src) = unsafe {
                let src = self.get(src_index) as *const _;
                let dst = self.get_mut(dst_index);
                let src: &SlotWrapper<_> = &*src;
                (dst, src)
            };

            memory.copy_slot(&mut dst.slot, &src.slot);
            dst.frame = src.frame;
        }
    }

    /// Advance the base slot's frame, returning its new frame.
    ///
    /// The base slot's frame must not equal Frame::Unknown.
    pub(crate) fn advance_base_slot(&mut self, memory: &M) -> u32 {
        let base = &mut self.base;

        let new_frame;
        match base.frame {
            Frame::PowerOn => {
                new_frame = 0;
            }
            Frame::At(frame) => {
                memory.advance_base_slot(&mut base.slot);
                new_frame = frame + 1;
            }
            _ => panic!("base.frame = Frame::Unknown"),
        };

        base.frame = Frame::At(new_frame);

        new_frame
    }
}

impl<M: GameMemory> fmt::Debug for Slots<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Slots").finish_non_exhaustive()
    }
}
