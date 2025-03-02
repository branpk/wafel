//! The main SM64 API for Wafel.
//!
//! This crate provides two simulation APIs: [Game] and [Timeline].
//!
//! - The [Game] API uses a traditional frame advance / save state model.
//!
//! - The [Timeline] API is higher level and manages save states internally. It allows accessing
//! arbitrary frames in any order. This is the API used by the Wafel application for its
//! rewind functionality.
//!
//! For now, I recommend using the [Game] API for brute forcing purposes. The [Timeline]
//! API works, but the algorithm is currently optimized for Wafel's UI, so it may not be as
//! fast in a brute forcing setting.
//!
//! The [Emu] API attaches to a running emulator and allows reading/writing to its process
//! memory. Similarly, the [RemoteDll] API attaches to a running instance of libsm64
//! in another process.
//!
//! # Data paths
//!
//! Many API methods take in a string `path` parameter. This string uses a C-like syntax to
//! denote a location in memory.
//!
//! The following are all valid syntax:
//! - `gMarioState.pos`
//! - `gObjectPool[12].oPosX`
//! - `gMarioState[0].wall?.normal`
//! - `gMarioState.action & ACT_ID_MASK`
//!
//! Note:
//! - `p.x` automatically dereferences a pointer `p`. The syntax `p->x` can also be used.
//! - `*` is not used for pointer dereferencing. Instead you can use `[0]`, `->`, or `.`.
//! - A mask can be applied using `&`. The mask must be an integer literal or constant name.
//! - Array indices must be an integer literal or constant name.
//! - `?` denotes that a pointer may be null. If so, the entire expression returns `Value::None`.
//!   If `?` is not used, an error is thrown instead.
//!
//! Variable and constant names are automatically pulled from the decomp source code.
//! However, the names may be different if Wafel is out of date from decomp.
//!
//! Object fields such as `oPosX` are supported and can be accessed using the regular `.`
//! syntax.

#![warn(missing_docs, missing_debug_implementations)]

pub use data_type::*;
pub use emu::*;
pub use error::*;
pub use game::*;
pub use lock::*;
pub use m64::*;
pub use remote_dll::*;
pub use timeline::*;
pub use wafel_data_type::{
    Address, Angle, FloatType, FloatValue, IntType, IntValue, Value, ValueTypeError,
};
pub use wafel_sm64::{ObjectHitbox, Surface};
pub use wafel_viz::VizScene;
pub use wafel_viz_sm64::{
    Camera, Element, InGameRenderMode, Line, LookAtCamera, ObjectCull, OrthoCamera, Point,
    SurfaceMode, VizConfig,
};

mod data_cache;
mod data_type;
mod emu;
mod error;
mod game;
mod lock;
mod m64;
mod remote_dll;
mod timeline;
