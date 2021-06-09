//! The Wafel timeline algorithm, which allows its TAS rewind functionality.
//!
//! There are three components to the timeline:
//! - A [GameMemory](wafel_memory::GameMemory) instance which implements basic frame advance
//!   and save state functionality
//! - A [GameController] which decides what data edits to make on each frame (e.g. what
//!   inputs to set)
//! - [GameTimeline] which owns the above components and provides the ability to access
//!   the game static at arbitrary frames
//!
//! # Note on frame numbers
//!
//! Wafel applies edits on frame i _after_ the game updates from frame i - 1 to frame i.
//!
//! For example, suppose A is pressed on frame 0, and B is pressed on frame 1.
//! Then the game will begin in its power-on state (frame 0), and then apply the "press A" edit.
//! The game will respond to the A press during the next frame advance 0 -> 1.
//! Then the "press B" edit will be applied.
//! The game state on frame i _includes_ the frame i edits - i.e. frame 0 includes the A press
//! and frame 1 includes the B press.
//!
//! This behavior differs from how Mupen loads inputs from a TAS. However, because Mupen has
//! input buffering and Wafel doesn't, the input frame indices happen to line up.
//! Note however that Wafel allows unbuffered inputs on frame 0, which isn't possible on an
//! emulator or real hardware.

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

pub use controller::*;
pub use timeline::*;

mod controller;
mod slots;
mod timeline;
