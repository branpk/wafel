//! Implementation of range editing (drag and drop in the frame sheet).

use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::Range,
};

/// A unique identifier for an edit range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EditRangeId(pub(crate) usize);

/// A range of contiguous cells in a single column which are edited to the same value.
#[derive(Debug, Clone)]
pub struct EditRange<V> {
    /// The id of the range.
    pub id: EditRangeId,
    /// The frames included in the range.
    pub frames: Range<u32>,
    /// The value that each variable in the range is edited to.
    pub value: V,
}

#[derive(Debug, Clone)]
pub(crate) enum EditOperation<C, V> {
    Write(C, u32, V),
    Reset(C, u32),
    Insert(u32),
    Delete(u32),
}

/// Manages all of the active edit ranges.
#[derive(Debug)]
pub(crate) struct RangeEdits<C, V> {
    ranges: HashMap<C, Ranges<V>>,
    drag_state: Option<DragState<C, V>>,
    next_range_id: usize,
}

impl<C: Hash + Eq + Clone, V: Clone> RangeEdits<C, V> {
    /// An empty set of edit ranges.
    pub(crate) fn new() -> Self {
        Self {
            ranges: HashMap::new(),
            drag_state: None,
            next_range_id: 0,
        }
    }

    /// Find all the edits for a given frame, across columns.
    pub(crate) fn edits(&self, frame: u32) -> Vec<(&C, &V)> {
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
    pub(crate) fn write(&mut self, column: &C, frame: u32, value: V) -> Vec<EditOperation<C, V>> {
        let mut ops = self.rollback_drag();

        let ranges = self.ranges.entry(column.clone()).or_default();
        let frames_to_update = ranges.set_value_or_create_range(
            frame,
            value,
            range_id_generator(&mut self.next_range_id),
        );
        ops.extend(self.ops_for_update(column, frames_to_update));

        ops
    }

    /// Reset the value for a given cell.
    ///
    /// If the cell is in an edit range, the edit range is split into two.
    pub(crate) fn reset(&mut self, column: &C, frame: u32) -> Vec<EditOperation<C, V>> {
        match self.find_range(&column, frame) {
            Some(range) => {
                let range = range.clone();
                let mut ops = self.rollback_drag();

                let ranges = self.ranges.entry(column.clone()).or_default();

                // Simulate a reset by dragging the cell up or down.
                let mut preview = RangeEditPreview::new(
                    frame,
                    range.value,
                    range_id_generator(&mut self.next_range_id),
                );
                let frames_to_update = preview.reset_source(ranges);
                preview.commit(ranges);
                ops.extend(self.ops_for_update(column, frames_to_update));

                ops
            }
            None => Vec::new(),
        }
    }

    /// Insert a frame, shifting all lower rows downward.
    pub(crate) fn insert_frame(&mut self, frame: u32) -> Vec<EditOperation<C, V>> {
        let mut ops = self.rollback_drag();
        for range in self.ranges.values_mut() {
            range.insert(frame, 1);
        }
        ops.push(EditOperation::Insert(frame));
        ops
    }

    /// Delete a frame, shifting all lower frames upward.
    pub(crate) fn delete_frame(&mut self, frame: u32) -> Vec<EditOperation<C, V>> {
        let mut ops = self.rollback_drag();
        for range in self.ranges.values_mut() {
            range.remove(frame, 1);
        }
        ops.push(EditOperation::Delete(frame));
        ops
    }

    /// Find the edit range containing a cell.
    pub(crate) fn find_range(&self, column: &C, frame: u32) -> Option<&EditRange<V>> {
        let ranges = self.ranges.get(column);
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
    pub(crate) fn begin_drag(
        &mut self,
        column: &C,
        source_frame: u32,
        source_value: V,
    ) -> Vec<EditOperation<C, V>> {
        let ops = self.rollback_drag();

        self.drag_state = Some(DragState {
            column: column.clone(),
            preview: RangeEditPreview::new(
                source_frame,
                source_value,
                range_id_generator(&mut self.next_range_id),
            ),
        });

        ops
    }

    /// Drag from `source_variable` to `target_frame`.
    ///
    /// The ranges will appear to be updated, but won't be committed until `release_drag` is
    /// called.
    pub(crate) fn update_drag(&mut self, target_frame: u32) -> Vec<EditOperation<C, V>> {
        if let Some(DragState { column, preview }) = &mut self.drag_state {
            let column = column.clone();
            let ranges = self.ranges.entry(column.clone()).or_default();
            let frames_to_update = preview.update_drag_target(&ranges, target_frame);
            self.ops_for_update(&column, frames_to_update)
        } else {
            Vec::new()
        }
    }

    /// End the drag operation, committing range changes.
    pub(crate) fn release_drag(&mut self) -> Vec<EditOperation<C, V>> {
        if let Some(DragState { column, preview }) = self.drag_state.take() {
            let ranges = self.ranges.entry(column).or_default();
            preview.commit(ranges);
        }
        Vec::new()
    }

    fn rollback_drag(&mut self) -> Vec<EditOperation<C, V>> {
        if let Some(DragState { column, preview }) = self.drag_state.take() {
            self.ops_for_update(&column, preview.rollback())
        } else {
            Vec::new()
        }
    }

    fn ops_for_update(&self, column: &C, frames: FramesToUpdate) -> Vec<EditOperation<C, V>> {
        Range::from(frames)
            .map(|frame| match self.find_range(column, frame) {
                Some(range) => EditOperation::Write(column.clone(), frame, range.value.clone()),
                None => EditOperation::Reset(column.clone(), frame),
            })
            .collect()
    }
}

fn range_id_generator(next_range_id: &mut usize) -> impl FnMut() -> EditRangeId + '_ {
    move || {
        let range_id = EditRangeId(*next_range_id);
        *next_range_id += 1;
        range_id
    }
}

#[derive(Debug)]
struct DragState<C, V> {
    column: C,
    preview: RangeEditPreview<V>,
}

#[derive(Debug, Clone)]
struct Ranges<V> {
    ranges: HashMap<EditRangeId, EditRange<V>>,
    ranges_by_frame: HashMap<u32, EditRangeId>,
}

impl<V> Ranges<V> {
    fn try_range(&self, range_id: EditRangeId) -> Option<&EditRange<V>> {
        self.ranges.get(&range_id)
    }

    fn range(&self, range_id: EditRangeId) -> &EditRange<V> {
        self.try_range(range_id).expect("invalid range id")
    }

    fn find_range_id(&self, frame: u32) -> Option<EditRangeId> {
        self.ranges_by_frame.get(&frame).cloned()
    }

    fn find_range(&self, frame: u32) -> Option<&EditRange<V>> {
        self.find_range_id(frame)
            .map(|range_id| self.range(range_id))
    }

    fn set_value_or_create_range(
        &mut self,
        frame: u32,
        value: V,
        mut gen_range_id: impl FnMut() -> EditRangeId,
    ) -> FramesToUpdate {
        match self.find_range_id(frame) {
            Some(range_id) => {
                let range = self.ranges.get_mut(&range_id).unwrap();
                range.value = value;
                range.frames.clone().into()
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
                (frame..frame + 1).into()
            }
        }
    }

    fn insert(&mut self, start_frame: u32, count: usize) -> Vec<EditOperation<(), V>> {
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
                self.ranges_by_frame.insert(frame, *range_id);
            }
        }

        (0..count)
            .map(|_| EditOperation::Insert(start_frame))
            .collect()
    }

    fn remove(&mut self, start_frame: u32, count: usize) -> Vec<EditOperation<(), V>> {
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

        (0..count)
            .map(|_| EditOperation::Delete(start_frame))
            .collect()
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

impl<V> Default for Ranges<V> {
    fn default() -> Self {
        Self {
            ranges: Default::default(),
            ranges_by_frame: Default::default(),
        }
    }
}

#[derive(Debug)]
struct RangeEditPreview<V> {
    drag_source: u32,
    source_value: V,
    ranges_override: HashMap<EditRangeId, EditRange<V>>,
    ranges_by_frame_override: HashMap<u32, Option<EditRangeId>>,
    reserved_range_id: EditRangeId,
    prev_drag_target: Option<u32>,
    frames_to_update: FramesToUpdate,
}

impl<V: Clone> RangeEditPreview<V> {
    fn new(
        drag_source: u32,
        source_value: V,
        mut gen_range_id: impl FnMut() -> EditRangeId,
    ) -> Self {
        Self {
            drag_source,
            source_value,
            ranges_override: HashMap::new(),
            ranges_by_frame_override: HashMap::new(),
            reserved_range_id: gen_range_id(),
            prev_drag_target: None,
            frames_to_update: FramesToUpdate::empty(),
        }
    }

    fn update_drag_target(&mut self, parent: &Ranges<V>, drag_target: u32) -> FramesToUpdate {
        if self.prev_drag_target == Some(drag_target) {
            return FramesToUpdate::empty();
        }
        self.prev_drag_target = Some(drag_target);

        self.ranges_override.clear();
        self.ranges_by_frame_override.clear();

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
                match drag_target.cmp(&self.drag_source) {
                    // Dragging upward from unedited cell
                    Ordering::Less => {
                        self.clear_frames_shrink_upward(parent, drag_target..self.drag_source + 1);
                        self.set_range(
                            parent,
                            self.reserved_range_id,
                            drag_target..self.drag_source + 1,
                            self.source_value.clone(),
                        );
                    }
                    // Dragging downward from unedited cell
                    Ordering::Greater => {
                        self.clear_frames_shrink_downward(
                            parent,
                            self.drag_source..drag_target + 1,
                        );
                        self.set_range(
                            parent,
                            self.reserved_range_id,
                            self.drag_source..drag_target + 1,
                            self.source_value.clone(),
                        );
                    }
                    Ordering::Equal => {}
                }
            }
        }

        self.frames_to_update.clone()
    }

    fn reset_source(&mut self, parent: &Ranges<V>) -> FramesToUpdate {
        self.ranges_override.clear();
        self.ranges_by_frame_override.clear();

        if let Some(existing_range_id) = parent.find_range_id(self.drag_source) {
            let existing_range = parent.range(existing_range_id);

            // Reset near top of range
            if self.drag_source
                < existing_range.frames.start + (existing_range.frames.len() / 2) as u32
            {
                self.set_range(
                    parent,
                    existing_range_id,
                    self.drag_source + 1..existing_range.frames.end,
                    existing_range.value.clone(),
                );
                self.set_range(
                    parent,
                    self.reserved_range_id,
                    existing_range.frames.start..self.drag_source,
                    existing_range.value.clone(),
                );
            }
            // Reset middle or bottom of range
            else {
                self.set_range(
                    parent,
                    existing_range_id,
                    existing_range.frames.start..self.drag_source,
                    existing_range.value.clone(),
                );
                self.set_range(
                    parent,
                    self.reserved_range_id,
                    self.drag_source + 1..existing_range.frames.end,
                    existing_range.value.clone(),
                );
            }
        }

        self.frames_to_update.clone()
    }

    fn range<'a>(&'a self, parent: &'a Ranges<V>, range_id: EditRangeId) -> &'a EditRange<V> {
        self.ranges_override
            .get(&range_id)
            .unwrap_or_else(|| parent.range(range_id))
    }

    fn find_range_id(&self, parent: &Ranges<V>, frame: u32) -> Option<EditRangeId> {
        self.ranges_by_frame_override
            .get(&frame)
            .cloned()
            .unwrap_or_else(|| parent.find_range_id(frame))
    }

    fn find_range<'a>(&'a self, parent: &'a Ranges<V>, frame: u32) -> Option<&EditRange<V>> {
        self.find_range_id(parent, frame)
            .map(|range_id| self.range(parent, range_id))
    }

    fn clear_frames_shrink_upward(&mut self, parent: &Ranges<V>, frames: Range<u32>) {
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

    fn clear_frames_shrink_downward(&mut self, parent: &Ranges<V>, frames: Range<u32>) {
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
        parent: &Ranges<V>,
        range_id: EditRangeId,
        frames: Range<u32>,
        value: V,
    ) {
        assert!(!(self.ranges_override.contains_key(&range_id)));

        if let Some(parent_range) = parent.try_range(range_id) {
            for frame in parent_range.frames.clone() {
                self.ranges_by_frame_override.insert(frame, None);
                self.frames_to_update.include(frame);
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
            self.frames_to_update.include(frame);
        }
    }

    fn commit(self, parent: &mut Ranges<V>) {
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

    fn rollback(self) -> FramesToUpdate {
        let mut frames_to_update = FramesToUpdate::empty();
        for frame in self.ranges_by_frame_override.keys() {
            frames_to_update.include(*frame);
        }
        for range in self.ranges_override.values() {
            frames_to_update.include_all(range.frames.clone());
        }
        frames_to_update
    }
}

#[derive(Debug, Clone, Default)]
#[must_use]
struct FramesToUpdate {
    start: u32,
    end: u32,
}

impl FramesToUpdate {
    fn empty() -> Self {
        Self::default()
    }

    fn include(&mut self, frame: u32) {
        if self.is_empty() {
            self.start = frame;
            self.end = frame + 1;
        } else {
            self.start = self.start.min(frame);
            self.end = self.end.max(frame + 1);
        }
    }

    fn include_all(&mut self, frames: impl Iterator<Item = u32>) {
        for frame in frames {
            self.include(frame);
        }
    }

    fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

impl From<Range<u32>> for FramesToUpdate {
    fn from(v: Range<u32>) -> Self {
        Self {
            start: v.start,
            end: v.end,
        }
    }
}

impl From<FramesToUpdate> for Range<u32> {
    fn from(v: FramesToUpdate) -> Self {
        v.start..v.end
    }
}
