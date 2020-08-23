//! Implementation of range editing (drag and drop in the frame sheet).

use super::Variable;
use crate::{memory::Value, timeline::InvalidatedFrames};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::Range,
};

/// A unique identifier for an edit range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EditRangeId(pub usize);

/// A range of contiguous cells in a single column which are edited to the same value.
#[derive(Debug, Clone)]
pub struct EditRange {
    /// The id of the range.
    pub id: EditRangeId,
    /// The frames included in the range.
    pub frames: Range<u32>,
    /// The value that each variable in the range is edited to.
    pub value: Value,
}

/// Manages all of the active edit ranges.
#[derive(Debug, Default)]
pub struct RangeEdits {
    ranges: HashMap<Variable, Ranges>,
    drag_state: Option<DragState>,
    next_range_id: usize,
}

impl RangeEdits {
    /// An empty set of edit ranges.
    pub fn new() -> Self {
        Default::default()
    }

    /// Find all the edits for a given frame, across columns.
    pub fn edits(&self, frame: u32) -> Vec<(&Variable, &Value)> {
        let mut edits = Vec::new();
        for column in self.ranges.keys() {
            if let Some(range) = self.find_range(column, frame) {
                edits.push((column, &range.value))
            }
        }
        edits
    }

    /// Edit the value of a given cell.
    ///
    /// If the cell is in an edit range, the entire edit range is given the
    /// value.
    /// Otherwise a new range containing only the cell is created.
    pub fn write(&mut self, column: &Variable, frame: u32, value: Value) -> InvalidatedFrames {
        let invalidated = self.rollback_drag();

        let ranges = self.ranges.entry(column.without_frame()).or_default();
        invalidated.union(ranges.set_value_or_create_range(
            frame,
            value,
            range_id_generator(&mut self.next_range_id),
        ))
    }

    /// Reset the value for a given cell.
    ///
    /// If the cell is in an edit range, the edit range is split into two.
    pub fn reset(&mut self, _column: &Variable, _frame: u32) -> InvalidatedFrames {
        let mut _invalidated = self.rollback_drag();
        todo!()
    }

    /// Insert a frame, shifting all lower rows downward.
    pub fn insert_frame(&mut self, frame: u32) -> InvalidatedFrames {
        let invalidated = self.rollback_drag();
        for range in self.ranges.values_mut() {
            range.insert(frame, 1);
        }
        invalidated.union(InvalidatedFrames::StartingAt(frame))
    }

    /// Delete a frame, shifting all lower frames upward.
    pub fn delete_frame(&mut self, frame: u32) -> InvalidatedFrames {
        let invalidated = self.rollback_drag();
        for range in self.ranges.values_mut() {
            range.remove(frame, 1);
        }
        invalidated.union(InvalidatedFrames::StartingAt(frame))
    }

    /// Find the edit range containing a cell.
    pub fn find_range(&self, column: &Variable, frame: u32) -> Option<&EditRange> {
        let ranges = self.ranges.get(&column.without_frame());
        ranges.and_then(|ranges| {
            if let Some(drag_state) = &self.drag_state {
                if &drag_state.column == column {
                    return drag_state.preview.find_range(ranges, frame);
                }
            }
            ranges.find_range(frame)
        })
    }

    /// Begin a drag operation in `column` starting at `source_frame`.
    pub fn begin_drag(
        &mut self,
        column: &Variable,
        source_frame: u32,
        source_value: &Value,
    ) -> InvalidatedFrames {
        let invalidated = self.rollback_drag();

        self.drag_state = Some(DragState {
            column: column.without_frame(),
            preview: RangeEditPreview::new(
                source_frame,
                source_value.clone(),
                range_id_generator(&mut self.next_range_id),
            ),
        });

        invalidated
    }

    /// Drag from `source_variable` to `target_frame`.
    ///
    /// The ranges will appear to be updated, but won't be committed until `release_drag` is
    /// called.
    pub fn update_drag(&mut self, target_frame: u32) -> InvalidatedFrames {
        if let Some(DragState { column, preview }) = &mut self.drag_state {
            let ranges = self.ranges.entry(column.without_frame()).or_default();
            preview.update_drag_target(&ranges, target_frame)
        } else {
            InvalidatedFrames::None
        }
    }

    /// End the drag operation, committing range changes.
    pub fn release_drag(&mut self) -> InvalidatedFrames {
        if let Some(DragState { column, preview }) = self.drag_state.take() {
            let ranges = self.ranges.entry(column.without_frame()).or_default();
            preview.commit(ranges);
        }
        InvalidatedFrames::None
    }

    fn rollback_drag(&mut self) -> InvalidatedFrames {
        if let Some(DragState { preview, .. }) = self.drag_state.take() {
            preview.rollback()
        } else {
            InvalidatedFrames::None
        }
    }
}

fn range_id_generator<'a>(next_range_id: &'a mut usize) -> impl FnMut() -> EditRangeId + 'a {
    move || {
        let range_id = EditRangeId(*next_range_id);
        *next_range_id += 1;
        range_id
    }
}

#[derive(Debug)]
struct DragState {
    column: Variable,
    preview: RangeEditPreview,
}

#[derive(Debug, Clone, Default)]
struct Ranges {
    ranges: HashMap<EditRangeId, EditRange>,
    ranges_by_frame: HashMap<u32, EditRangeId>,
}

impl Ranges {
    fn try_range(&self, range_id: EditRangeId) -> Option<&EditRange> {
        self.ranges.get(&range_id)
    }

    fn range(&self, range_id: EditRangeId) -> &EditRange {
        self.try_range(range_id).expect("invalid range id")
    }

    fn find_range_id(&self, frame: u32) -> Option<EditRangeId> {
        self.ranges_by_frame.get(&frame).cloned()
    }

    fn find_range(&self, frame: u32) -> Option<&EditRange> {
        self.find_range_id(frame)
            .map(|range_id| self.range(range_id))
    }

    fn set_value_or_create_range(
        &mut self,
        frame: u32,
        value: Value,
        mut gen_range_id: impl FnMut() -> EditRangeId,
    ) -> InvalidatedFrames {
        match self.find_range_id(frame) {
            Some(range_id) => {
                let range = self.ranges.get_mut(&range_id).unwrap();
                range.value = value;
                InvalidatedFrames::StartingAt(range.frames.start)
            }
            None => {
                let range_id = gen_range_id();
                self.ranges.insert(
                    range_id,
                    EditRange {
                        id: range_id,
                        frames: frame..frame + 1,
                        value,
                    },
                );
                self.ranges_by_frame.insert(frame, range_id);
                InvalidatedFrames::StartingAt(frame)
            }
        }
    }

    fn insert(&mut self, start_frame: u32, count: usize) {
        let shift = |frame| {
            if frame >= start_frame {
                frame + count as u32
            } else {
                frame
            }
        };

        self.ranges = self
            .ranges
            .drain()
            .map(|(range_id, mut range)| {
                range.frames = shift(range.frames.start)..shift(range.frames.end - 1) + 1;
                (range_id, range)
            })
            .collect();

        self.ranges_by_frame.clear();
        for (range_id, range) in &self.ranges {
            for frame in range.frames.clone() {
                self.ranges_by_frame.insert(frame, range_id.clone());
            }
        }
    }

    fn remove(&mut self, start_frame: u32, count: usize) {
        let shift = |frame| {
            if frame >= start_frame + count as u32 {
                Some(frame - count as u32)
            } else if frame >= start_frame {
                None
            } else {
                Some(frame)
            }
        };

        self.ranges_by_frame = self
            .ranges_by_frame
            .drain()
            .filter_map(|(frame, range_id)| shift(frame).map(|frame| (frame, range_id)))
            .collect();

        self.ranges = self
            .ranges
            .drain()
            .filter_map(|(range_id, mut range)| {
                let start = shift(range.frames.start).unwrap_or(start_frame);
                let end = shift(range.frames.end).unwrap_or(start_frame);
                range.frames = start..end;

                if range.frames.is_empty() {
                    None
                } else {
                    Some((range_id, range))
                }
            })
            .collect();
    }

    fn validate(&self) {
        for (range_id, range) in &self.ranges {
            assert!(!range.frames.is_empty());
            for frame in range.frames.clone() {
                assert!(self.ranges_by_frame.get(&frame) == Some(range_id));
            }
        }
        for (frame, range_id) in &self.ranges_by_frame {
            let range = self.ranges.get(range_id).expect("invalid range id");
            assert!(range.frames.contains(frame));
        }
    }
}

#[derive(Debug)]
struct RangeEditPreview {
    drag_source: u32,
    source_value: Value,
    ranges_override: HashMap<EditRangeId, EditRange>,
    ranges_by_frame_override: HashMap<u32, Option<EditRangeId>>,
    reserved_range_id: EditRangeId,
    prev_drag_target: Option<u32>,
    invalidated_frames: InvalidatedFrames,
}

impl RangeEditPreview {
    fn new(
        drag_source: u32,
        source_value: Value,
        mut gen_range_id: impl FnMut() -> EditRangeId,
    ) -> Self {
        Self {
            drag_source,
            source_value,
            ranges_override: HashMap::new(),
            ranges_by_frame_override: HashMap::new(),
            reserved_range_id: gen_range_id(),
            prev_drag_target: None,
            invalidated_frames: InvalidatedFrames::None,
        }
    }

    fn update_drag_target(&mut self, parent: &Ranges, drag_target: u32) -> InvalidatedFrames {
        if self.prev_drag_target == Some(drag_target) {
            return InvalidatedFrames::None;
        }
        self.prev_drag_target = Some(drag_target);

        self.ranges_override.clear();
        self.ranges_by_frame_override.clear();
        self.invalidated_frames.clear();

        match parent.find_range_id(self.drag_source) {
            Some(existing_range_id) => {
                let existing_range = parent.range(existing_range_id);

                // Dragging top of range
                if existing_range.frames.start == self.drag_source
                    && (existing_range.frames.len() > 1 || drag_target < self.drag_source)
                {
                    self.clear_frames_shrink_upward(
                        parent,
                        drag_target..existing_range.frames.start,
                    );
                    self.set_range(
                        parent,
                        existing_range_id,
                        drag_target..existing_range.frames.end,
                        existing_range.value.clone(),
                    );
                }
                // Dragging bottom of range
                else if existing_range.frames.end - 1 == self.drag_source
                    && (existing_range.frames.len() > 1 || drag_target > self.drag_source)
                {
                    self.clear_frames_shrink_downward(
                        parent,
                        existing_range.frames.end..drag_target + 1,
                    );
                    self.set_range(
                        parent,
                        existing_range_id,
                        existing_range.frames.start..drag_target + 1,
                        existing_range.value.clone(),
                    );
                }
                // Dragging upward from middle of range
                else if drag_target < self.drag_source {
                    self.set_range(
                        parent,
                        existing_range_id,
                        existing_range.frames.start..drag_target + 1,
                        existing_range.value.clone(),
                    );
                    self.set_range(
                        parent,
                        self.reserved_range_id,
                        self.drag_source + 1..existing_range.frames.end,
                        existing_range.value.clone(),
                    );
                }
                // Dragging downward from middle of range
                else if drag_target > self.drag_source {
                    self.set_range(
                        parent,
                        existing_range_id,
                        drag_target..existing_range.frames.end,
                        existing_range.value.clone(),
                    );
                    self.set_range(
                        parent,
                        self.reserved_range_id,
                        existing_range.frames.start..self.drag_source,
                        existing_range.value.clone(),
                    );
                }
            }
            None => {
                // Dragging upward from unedited cell
                if drag_target < self.drag_source {
                    self.clear_frames_shrink_upward(parent, drag_target..self.drag_source + 1);
                    self.set_range(
                        parent,
                        self.reserved_range_id,
                        drag_target..self.drag_source + 1,
                        self.source_value.clone(),
                    );
                }
                // Dragging downward from unedited cell
                else if drag_target > self.drag_source {
                    self.clear_frames_shrink_downward(parent, self.drag_source..drag_target + 1);
                    self.set_range(
                        parent,
                        self.reserved_range_id,
                        self.drag_source..drag_target + 1,
                        self.source_value.clone(),
                    );
                }
            }
        }

        self.invalidated_frames
    }

    fn range<'a>(&'a self, parent: &'a Ranges, range_id: EditRangeId) -> &'a EditRange {
        self.ranges_override
            .get(&range_id)
            .unwrap_or_else(|| parent.range(range_id))
    }

    fn find_range_id(&self, parent: &Ranges, frame: u32) -> Option<EditRangeId> {
        self.ranges_by_frame_override
            .get(&frame)
            .cloned()
            .unwrap_or_else(|| parent.find_range_id(frame))
    }

    fn find_range<'a>(&'a self, parent: &'a Ranges, frame: u32) -> Option<&EditRange> {
        self.find_range_id(parent, frame)
            .map(|range_id| self.range(parent, range_id))
    }

    fn clear_frames_shrink_upward(&mut self, parent: &Ranges, frames: Range<u32>) {
        let range_ids: HashSet<EditRangeId> = frames
            .clone()
            .flat_map(|frame| self.find_range_id(parent, frame))
            .collect();

        for range_id in range_ids {
            let range = self.range(parent, range_id);
            let new_frames = range.frames.start..frames.start;
            let value = range.value.clone();
            self.set_range(parent, range_id, new_frames, value);
        }
    }

    fn clear_frames_shrink_downward(&mut self, parent: &Ranges, frames: Range<u32>) {
        let range_ids: HashSet<EditRangeId> = frames
            .clone()
            .flat_map(|frame| self.find_range_id(parent, frame))
            .collect();

        for range_id in range_ids {
            let range = self.range(parent, range_id);
            let new_frames = frames.end..range.frames.end;
            let value = range.value.clone();
            self.set_range(parent, range_id, new_frames, value);
        }
    }

    fn set_range(
        &mut self,
        parent: &Ranges,
        range_id: EditRangeId,
        frames: Range<u32>,
        value: Value,
    ) {
        assert!(!(self.ranges_override.contains_key(&range_id)));

        if let Some(parent_range) = parent.try_range(range_id) {
            for frame in parent_range.frames.clone() {
                self.ranges_by_frame_override.insert(frame, None);
                self.invalidated_frames.include(frame);
            }
        }

        self.ranges_override.insert(
            range_id,
            EditRange {
                id: range_id,
                frames: frames.clone(),
                value,
            },
        );
        for frame in frames {
            self.ranges_by_frame_override.insert(frame, Some(range_id));
            self.invalidated_frames.include(frame);
        }
    }

    fn commit(&self, parent: &mut Ranges) {
        for (&frame, &range_id) in &self.ranges_by_frame_override {
            match range_id {
                Some(range_id) => {
                    parent.ranges_by_frame.insert(frame, range_id);
                }
                None => {
                    parent.ranges_by_frame.remove(&frame);
                }
            }
        }

        for (range_id, range) in &self.ranges_override {
            if range.frames.is_empty() {
                parent.ranges.remove(range_id);
            } else {
                parent.ranges.insert(*range_id, range.clone());
            }
        }

        parent.validate();
    }

    fn rollback(&self) -> InvalidatedFrames {
        let mut invalidated_frames = InvalidatedFrames::None;
        for frame in self.ranges_by_frame_override.keys() {
            invalidated_frames.include(*frame);
        }
        for range in self.ranges_override.values() {
            invalidated_frames.include(range.frames.start);
        }
        invalidated_frames
    }
}
