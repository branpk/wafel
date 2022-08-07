#![allow(missing_docs)]

use core::fmt;
use std::error::Error;

use wafel_data_access::DataError;
use wafel_data_type::{IntValue, ValueTypeError};
use wafel_layout::LayoutLookupError;
use wafel_memory::MemoryError;

#[derive(Debug, Clone)]
pub enum SM64DataError {
    DataError(DataError),
    InvalidFrameLogEventType(IntValue),
    UnsizedSurfacePoolPointer,
    UnsizedObjectPoolArray,
}

impl fmt::Display for SM64DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SM64DataError::DataError(error) => write!(f, "{}", error),
            SM64DataError::InvalidFrameLogEventType(value) => {
                write!(f, "invalid frame log event type: {}", value)
            }
            SM64DataError::UnsizedSurfacePoolPointer => {
                write!(f, "surface pool array does not have a stride")
            }
            SM64DataError::UnsizedObjectPoolArray => {
                write!(f, "object pool array does not have a stride")
            }
        }
    }
}

impl Error for SM64DataError {}

impl From<DataError> for SM64DataError {
    fn from(v: DataError) -> Self {
        Self::DataError(v)
    }
}

impl From<ValueTypeError> for SM64DataError {
    fn from(v: ValueTypeError) -> Self {
        Self::DataError(v.into())
    }
}

impl From<LayoutLookupError> for SM64DataError {
    fn from(v: LayoutLookupError) -> Self {
        Self::DataError(v.into())
    }
}

impl From<MemoryError> for SM64DataError {
    fn from(v: MemoryError) -> Self {
        Self::DataError(v.into())
    }
}
