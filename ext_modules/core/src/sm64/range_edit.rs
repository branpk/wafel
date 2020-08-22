use super::Variable;
use crate::{error::Error, memory::Value};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::Range,
};

#[derive(Debug, Default)]
pub struct VariableRangeEdits {
    ranges: HashMap<Variable, Ranges>,
    values: HashMap<Variable, HashMap<RangeId, Value>>,
    drag_state: Option<DragState>,
}

impl VariableRangeEdits {
    pub fn new() -> Self {
        Self {
            ranges: HashMap::new(),
            values: HashMap::new(),
            drag_state: None,
        }
    }

    pub fn write(&mut self, variable: &Variable, value: &Value) -> Result<(), Error> {
        self.rollback_drag();

        let column = variable.without_frame();
        let frame = variable.try_frame()?;

        let ranges = self.ranges.entry(column.clone()).or_default();
        let values = self.values.entry(column.clone()).or_default();

        let range_id = ranges.find_or_create_range(frame as usize);
        values.insert(range_id, value.clone());

        Ok(())
    }

    pub fn range_key(&self, variable: &Variable) -> Result<Option<usize>, Error> {
        let column = variable.without_frame();
        let frame = variable.try_frame()?;

        match self.ranges.get(&column) {
            Some(ranges) => {
                if let Some(drag_state) = &self.drag_state {
                    if drag_state.column == column {
                        return Ok(drag_state
                            .preview
                            .find_range(ranges, frame as usize)
                            .map(|range_id| range_id.0));
                    }
                }
                Ok(ranges.find_range(frame as usize).map(|range_id| range_id.0))
            }
            None => Ok(None),
        }
    }

    pub fn begin_drag(
        &mut self,
        source_variable: &Variable,
        source_value: &Value,
    ) -> Result<(), Error> {
        self.rollback_drag();

        let column = source_variable.without_frame();
        let frame = source_variable.try_frame()?;

        let ranges = self.ranges.entry(column.clone()).or_default();

        self.drag_state = Some(DragState {
            column,
            source_value: source_value.clone(),
            preview: RangeEditPreview::new(ranges, frame as usize),
        });

        Ok(())
    }

    pub fn update_drag(&mut self, target: usize) {
        if let Some(DragState {
            column,
            source_value,
            preview,
        }) = &mut self.drag_state
        {
            let ranges = self.ranges.entry(column.clone()).or_default();
            let values = self.values.entry(column.clone()).or_default();

            let op = preview.update_drag_target(&ranges, target);

            // Initialize value of any new ranges
            if let Some(SetRangeValue { range_id, op }) = op {
                let value = match op {
                    SetRangeValueOp::SourceValue => source_value.clone(),
                    SetRangeValueOp::ValueOf(other_range_id) => values
                        .get(&other_range_id)
                        .expect("invalid range id")
                        .clone(),
                };
                values.insert(range_id, value);
            }
        }
    }

    pub fn release_drag(&mut self) {
        if let Some(DragState {
            column, preview, ..
        }) = self.drag_state.take()
        {
            let ranges = self.ranges.entry(column.clone()).or_default();
            let values = self.values.entry(column).or_default();

            let deleted_ranges = preview.commit(ranges);
            for range_id in deleted_ranges {
                values.remove(&range_id);
            }
        }
    }

    fn rollback_drag(&mut self) {
        if let Some(DragState {
            column, preview, ..
        }) = self.drag_state.take()
        {
            let values = self.values.entry(column).or_default();

            let deleted_ranges = preview.rollback();
            for range_id in deleted_ranges {
                values.remove(&range_id);
            }
        }
    }

    pub fn insert(&mut self, start_index: usize, count: usize) {
        self.rollback_drag();
        for range in self.ranges.values_mut() {
            range.insert(start_index, count);
        }
    }

    pub fn remove(&mut self, start_index: usize, count: usize) {
        self.rollback_drag();
        for range in self.ranges.values_mut() {
            range.insert(start_index, count);
        }
    }
}

#[derive(Debug)]
struct DragState {
    column: Variable,
    source_value: Value,
    preview: RangeEditPreview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SetRangeValue {
    range_id: RangeId,
    op: SetRangeValueOp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SetRangeValueOp {
    SourceValue,
    ValueOf(RangeId),
}

#[derive(Debug, Clone, Default)]
struct Ranges {
    ranges: HashMap<RangeId, Range<usize>>,
    ranges_by_index: HashMap<usize, RangeId>,
    next_range_id: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RangeId(usize);

impl Ranges {
    fn try_range(&self, range_id: RangeId) -> Option<Range<usize>> {
        self.ranges.get(&range_id).cloned()
    }

    fn range(&self, range_id: RangeId) -> Range<usize> {
        self.try_range(range_id).expect("invalid range id")
    }

    fn find_range(&self, index: usize) -> Option<RangeId> {
        self.ranges_by_index.get(&index).cloned()
    }

    fn find_or_create_range(&mut self, index: usize) -> RangeId {
        match self.find_range(index) {
            Some(range_id) => range_id,
            None => {
                let range_id = self.reserve_range_id();
                self.ranges.insert(range_id, index..index + 1);
                self.ranges_by_index.insert(index, range_id);
                range_id
            }
        }
    }

    fn insert(&mut self, start_index: usize, count: usize) {
        todo!()
    }

    fn remove(&mut self, start_index: usize, count: usize) {
        todo!()
    }

    fn reserve_range_id(&mut self) -> RangeId {
        let range_id = RangeId(self.next_range_id);
        self.next_range_id += 1;
        range_id
    }

    fn validate(&self) {
        for (range_id, range) in &self.ranges {
            assert!(!range.is_empty());
            for index in range.clone() {
                assert!(self.ranges_by_index.get(&index) == Some(range_id));
            }
        }
        for (index, range_id) in &self.ranges_by_index {
            let range = self.ranges.get(range_id).expect("invalid range id");
            assert!(range.contains(index));
        }
    }
}

#[derive(Debug)]
struct RangeEditPreview {
    drag_source: usize,
    ranges_override: HashMap<RangeId, Range<usize>>,
    ranges_by_index_override: HashMap<usize, Option<RangeId>>,
    reserved_range_id: RangeId,
}

impl RangeEditPreview {
    fn new(parent: &mut Ranges, drag_source: usize) -> Self {
        Self {
            drag_source,
            ranges_override: HashMap::new(),
            ranges_by_index_override: HashMap::new(),
            reserved_range_id: parent.reserve_range_id(),
        }
    }

    fn update_drag_target(&mut self, parent: &Ranges, drag_target: usize) -> Option<SetRangeValue> {
        self.ranges_override.clear();
        self.ranges_by_index_override.clear();

        match parent.find_range(self.drag_source) {
            Some(existing_range_id) => {
                let existing_range = parent.range(existing_range_id);

                // Dragging top of range
                if existing_range.start == self.drag_source {
                    self.clear_indices_shrink_upward(parent, drag_target..existing_range.start);
                    self.override_range(parent, existing_range_id, drag_target..existing_range.end);
                    None
                }
                // Dragging bottom of range
                else if existing_range.end - 1 == self.drag_source {
                    self.clear_indices_shrink_downward(parent, existing_range.end..drag_target + 1);
                    self.override_range(
                        parent,
                        existing_range_id,
                        existing_range.start..drag_target + 1,
                    );
                    None
                }
                // Dragging upward from middle of range
                else if drag_target < self.drag_source {
                    self.override_range(
                        parent,
                        existing_range_id,
                        existing_range.start..drag_target + 1,
                    );
                    self.override_range(
                        parent,
                        self.reserved_range_id,
                        self.drag_source + 1..existing_range.end,
                    );
                    Some(SetRangeValue {
                        range_id: self.reserved_range_id,
                        op: SetRangeValueOp::ValueOf(existing_range_id),
                    })
                }
                // Dragging downward from middle of range
                else if drag_target > self.drag_source {
                    self.override_range(parent, existing_range_id, drag_target..existing_range.end);
                    self.override_range(
                        parent,
                        self.reserved_range_id,
                        existing_range.start..self.drag_source,
                    );
                    Some(SetRangeValue {
                        range_id: self.reserved_range_id,
                        op: SetRangeValueOp::ValueOf(existing_range_id),
                    })
                }
                // Stationary
                else {
                    None
                }
            }
            None => {
                // Dragging upward from unedited cell
                if drag_target < self.drag_source {
                    self.override_range(
                        parent,
                        self.reserved_range_id,
                        drag_target..self.drag_source + 1,
                    );
                    Some(SetRangeValue {
                        range_id: self.reserved_range_id,
                        op: SetRangeValueOp::SourceValue,
                    })
                }
                // Dragging downward from unedited cell
                else if drag_target > self.drag_source {
                    self.override_range(
                        parent,
                        self.reserved_range_id,
                        self.drag_source..drag_target + 1,
                    );
                    Some(SetRangeValue {
                        range_id: self.reserved_range_id,
                        op: SetRangeValueOp::SourceValue,
                    })
                }
                // Stationary
                else {
                    None
                }
            }
        }
    }

    fn find_range(&self, parent: &Ranges, index: usize) -> Option<RangeId> {
        self.ranges_by_index_override
            .get(&index)
            .cloned()
            .unwrap_or_else(|| parent.find_range(index))
    }

    fn range(&self, parent: &Ranges, range_id: RangeId) -> Range<usize> {
        self.ranges_override
            .get(&range_id)
            .cloned()
            .unwrap_or_else(|| parent.range(range_id))
    }

    fn clear_indices_shrink_upward(&mut self, parent: &Ranges, indices: Range<usize>) {
        let range_ids: HashSet<RangeId> = indices
            .clone()
            .flat_map(|index| self.find_range(parent, index))
            .collect();

        for range_id in range_ids {
            let range = self.range(parent, range_id);
            self.override_range(parent, range_id, range.start..indices.start);
        }
    }

    fn clear_indices_shrink_downward(&mut self, parent: &Ranges, indices: Range<usize>) {
        let range_ids: HashSet<RangeId> = indices
            .clone()
            .flat_map(|index| self.find_range(parent, index))
            .collect();

        for range_id in range_ids {
            let range = self.range(parent, range_id);
            self.override_range(parent, range_id, indices.end..range.end);
        }
    }

    fn override_range(&mut self, parent: &Ranges, range_id: RangeId, new_range: Range<usize>) {
        assert!(!(self.ranges_override.contains_key(&range_id)));

        if let Some(parent_range) = parent.try_range(range_id) {
            for index in parent_range.clone() {
                self.ranges_by_index_override.insert(index, None);
            }
        }

        self.ranges_override.insert(range_id, new_range);
    }

    fn commit(&self, parent: &mut Ranges) -> HashSet<RangeId> {
        for (&index, &range_id) in &self.ranges_by_index_override {
            match range_id {
                Some(range_id) => {
                    parent.ranges_by_index.insert(index, range_id);
                }
                None => {
                    parent.ranges_by_index.remove(&index);
                }
            }
        }

        let mut deleted_ranges = HashSet::new();

        for (range_id, range) in &self.ranges_override {
            if range.is_empty() {
                parent.ranges.remove(range_id);
                deleted_ranges.insert(*range_id);
            } else {
                parent.ranges.insert(*range_id, range.clone());
            }
        }

        parent.validate();

        deleted_ranges
    }

    fn rollback(&self) -> HashSet<RangeId> {
        let mut deleted_ranges = HashSet::new();
        deleted_ranges.insert(self.reserved_range_id);
        deleted_ranges
    }
}
