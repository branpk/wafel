use super::Variable;
use crate::{error::Error, memory::Value};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::Range,
};

#[derive(Debug, Default)]
pub struct RangeEdits {
    ranges: HashMap<Variable, Ranges>,
    drag_state: Option<DragState>,
}

impl RangeEdits {
    pub fn new() -> Self {
        Self {
            ranges: HashMap::new(),
            drag_state: None,
        }
    }

    pub fn edits(&self, frame: u32) -> Vec<(&Variable, &Value)> {
        let mut edits = Vec::new();
        for column in self.ranges.keys() {
            if let Some(range) = self.find_range(column, frame) {
                edits.push((column, &range.value))
            }
        }
        edits
    }

    pub fn write(&mut self, variable: &Variable, value: Value) -> Result<(), Error> {
        self.rollback_drag();

        let column = variable.without_frame();
        let frame = variable.try_frame()?;

        let ranges = self.ranges.entry(column.clone()).or_default();
        ranges.set_value_or_create_range(frame, value);

        Ok(())
    }

    pub fn reset(&mut self, variable: &Variable) -> Result<(), Error> {
        self.rollback_drag();

        todo!()
    }

    pub fn insert_frame(&mut self, frame: u32) {
        self.rollback_drag();
        for range in self.ranges.values_mut() {
            range.insert(frame, 1);
        }
    }

    pub fn delete_frame(&mut self, frame: u32) {
        self.rollback_drag();
        for range in self.ranges.values_mut() {
            range.remove(frame, 1);
        }
    }

    fn find_range(&self, column: &Variable, frame: u32) -> Option<&EditRange> {
        self.ranges.get(column).and_then(|ranges| {
            if let Some(drag_state) = &self.drag_state {
                if &drag_state.column == column {
                    return drag_state.preview.find_range(ranges, frame);
                }
            }
            ranges.find_range(frame)
        })
    }

    pub fn range_key(&self, variable: &Variable) -> Result<Option<usize>, Error> {
        Ok(self
            .find_range(&variable.without_frame(), variable.try_frame()?)
            .map(|range| range.id.0))
    }

    pub fn range_min(&self, variable: &Variable) -> Result<u32, Error> {
        Ok(self
            .find_range(&variable.without_frame(), variable.try_frame()?)
            .map(|range| range.frames.start)
            .unwrap_or(variable.try_frame()?))
    }

    pub fn begin_drag(
        &mut self,
        source_variable: &Variable,
        source_value: &Value,
    ) -> Result<(), Error> {
        self.rollback_drag();

        let column = source_variable.without_frame();
        let ranges = self.ranges.entry(column.clone()).or_default();

        self.drag_state = Some(DragState {
            column,
            preview: RangeEditPreview::new(
                ranges,
                source_variable.try_frame()?,
                source_value.clone(),
            ),
        });

        Ok(())
    }

    pub fn update_drag(&mut self, target_frame: u32) {
        if let Some(DragState { column, preview }) = &mut self.drag_state {
            let ranges = self.ranges.entry(column.clone()).or_default();
            preview.update_drag_target(&ranges, target_frame);
        }
    }

    pub fn release_drag(&mut self) {
        if let Some(DragState { column, preview }) = self.drag_state.take() {
            let ranges = self.ranges.entry(column.clone()).or_default();
            preview.commit(ranges);
        }
    }

    fn rollback_drag(&mut self) {
        self.drag_state = None;
    }
}

#[derive(Debug)]
struct DragState {
    column: Variable,
    preview: RangeEditPreview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EditRangeId(usize);

#[derive(Debug, Clone)]
struct EditRange {
    id: EditRangeId,
    frames: Range<u32>,
    value: Value,
}

#[derive(Debug, Clone, Default)]
struct Ranges {
    ranges: HashMap<EditRangeId, EditRange>,
    ranges_by_frame: HashMap<u32, EditRangeId>,
    next_range_id: usize,
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

    fn set_value_or_create_range(&mut self, frame: u32, value: Value) {
        match self.find_range_id(frame) {
            Some(range_id) => {
                self.ranges.get_mut(&range_id).unwrap().value = value;
            }
            None => {
                let range_id = self.reserve_range_id();
                self.ranges.insert(
                    range_id,
                    EditRange {
                        id: range_id,
                        frames: frame..frame + 1,
                        value,
                    },
                );
                self.ranges_by_frame.insert(frame, range_id);
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

        self.ranges_by_frame = self
            .ranges_by_frame
            .drain()
            .map(|(frame, range_id)| (shift(frame), range_id))
            .collect();

        self.ranges = self
            .ranges
            .drain()
            .map(|(range_id, mut range)| {
                range.frames = shift(range.frames.start)..shift(range.frames.end - 1) + 1;
                (range_id, range)
            })
            .collect();
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

    fn reserve_range_id(&mut self) -> EditRangeId {
        let range_id = EditRangeId(self.next_range_id);
        self.next_range_id += 1;
        range_id
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
}

impl RangeEditPreview {
    fn new(parent: &mut Ranges, drag_source: u32, source_value: Value) -> Self {
        Self {
            drag_source,
            source_value,
            ranges_override: HashMap::new(),
            ranges_by_frame_override: HashMap::new(),
            reserved_range_id: parent.reserve_range_id(),
        }
    }

    fn update_drag_target(&mut self, parent: &Ranges, drag_target: u32) {
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
            self.set_range(
                parent,
                range_id,
                range.frames.start..frames.start,
                range.value.clone(),
            );
        }
    }

    fn clear_frames_shrink_downward(&mut self, parent: &Ranges, frames: Range<u32>) {
        let range_ids: HashSet<EditRangeId> = frames
            .clone()
            .flat_map(|frame| self.find_range_id(parent, frame))
            .collect();

        for range_id in range_ids {
            let range = self.range(parent, range_id);
            self.set_range(
                parent,
                range_id,
                frames.end..range.frames.end,
                range.value.clone(),
            );
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
}
