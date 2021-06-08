use wafel_memory::GameMemory;

/// Applies edits at the end of each frame to control the game.
pub trait GameController<M: GameMemory> {
    type Error;

    /// Apply edits to the given state.
    fn apply(&self, memory: &M, slot: &mut M::Slot, frame: u32) -> Result<(), Self::Error>;
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
