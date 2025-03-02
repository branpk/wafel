#![allow(missing_docs)]

use core::fmt;
use std::error::Error;

use fast3d::F3DError;
use wafel_data_access::DataError;
use wafel_data_type::{DataTypeError, ValueTypeError};
use wafel_layout::LayoutLookupError;
use wafel_memory::MemoryError;
use wafel_sm64::SM64DataError;

#[derive(Debug, Clone)]
pub enum VizError {
    DataError(DataError),
    SM64DataError(SM64DataError),
    F3DError(F3DError),
    UnexpectedDisplayListCommand,
    MasterListDiscrepancy { descr: String },
    InvalidF3DPointer,
    InvalidGfxTree { descr: &'static str },
}

impl fmt::Display for VizError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VizError::DataError(error) => write!(f, "{}", error),
            VizError::SM64DataError(error) => write!(f, "{}", error),
            VizError::F3DError(error) => write!(f, "{}", error),
            VizError::UnexpectedDisplayListCommand => {
                write!(f, "unexpected display list command (wafel bug)")
            }
            VizError::MasterListDiscrepancy { descr } => write!(
                f,
                "unexpected display list in master list (wafel bug). Info: {}",
                descr
            ),
            VizError::InvalidF3DPointer => {
                write!(f, "invalid pointer while processing display list")
            }
            VizError::InvalidGfxTree { descr } => write!(f, "invalid gfx tree: {}", descr),
        }
    }
}

impl Error for VizError {}

impl From<DataError> for VizError {
    fn from(v: DataError) -> Self {
        Self::DataError(v)
    }
}

impl From<MemoryError> for VizError {
    fn from(v: MemoryError) -> Self {
        Self::DataError(v.into())
    }
}

impl From<LayoutLookupError> for VizError {
    fn from(v: LayoutLookupError) -> Self {
        Self::DataError(v.into())
    }
}

impl From<DataTypeError> for VizError {
    fn from(v: DataTypeError) -> Self {
        Self::DataError(v.into())
    }
}

impl From<ValueTypeError> for VizError {
    fn from(v: ValueTypeError) -> Self {
        Self::DataError(v.into())
    }
}

impl From<SM64DataError> for VizError {
    fn from(v: SM64DataError) -> Self {
        Self::SM64DataError(v)
    }
}

impl From<F3DError> for VizError {
    fn from(v: F3DError) -> Self {
        Self::F3DError(v)
    }
}
