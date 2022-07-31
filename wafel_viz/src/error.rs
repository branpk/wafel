#![allow(missing_docs)]

use core::fmt;
use std::error::Error;

use wafel_data_access::DataError;
use wafel_data_type::{DataTypeError, ValueTypeError};
use wafel_layout::LayoutLookupError;
use wafel_memory::MemoryError;
use wafel_sm64::SM64DataError;

#[derive(Debug, Clone)]
pub enum VizError {
    DataError(DataError),
    SM64DataError(SM64DataError),
    UnexpectedDisplayListCommand,
}

impl fmt::Display for VizError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VizError::DataError(error) => write!(f, "{}", error),
            VizError::SM64DataError(error) => write!(f, "{}", error),
            VizError::UnexpectedDisplayListCommand => {
                write!(f, "unexpected display list command (probably wafel bug)")
            }
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
