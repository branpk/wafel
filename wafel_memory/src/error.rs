use std::{error::Error, fmt};

use wafel_data_type::DataTypeRef;
use wafel_layout::{DllLayoutError, LayoutLookupError};

#[derive(Debug, Clone)]
pub enum MemoryError {
    Context {
        context: String,
        error: Box<MemoryError>,
    },
    LayoutLookupError(LayoutLookupError),
    UnreadableValue(DataTypeRef),
    UnwritableValue(DataTypeRef),
    InvalidAddress,
    WriteToStaticAddress,
    NonStaticAddressInStaticView,
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::Context { context, error } => write!(f, "{}: {}", context, error),
            MemoryError::LayoutLookupError(error) => write!(f, "{}", error),
            MemoryError::UnreadableValue(data_type) => {
                write!(f, "cannot read value of type {}", data_type)
            }
            MemoryError::UnwritableValue(data_type) => {
                write!(f, "cannot write value of type {}", data_type)
            }
            MemoryError::InvalidAddress => write!(f, "null or invalid address"),
            MemoryError::WriteToStaticAddress => write!(f, "write to static address"),
            MemoryError::NonStaticAddressInStaticView => {
                write!(f, "using a non-static address through a static memory view")
            }
        }
    }
}

impl Error for MemoryError {}

impl From<LayoutLookupError> for MemoryError {
    fn from(v: LayoutLookupError) -> Self {
        Self::LayoutLookupError(v)
    }
}

#[derive(Debug)]
pub enum DllLoadError {
    DlOpenError(dlopen::Error),
    DllLayoutError(DllLayoutError),
    UndefinedSymbol(String),
}

impl fmt::Display for DllLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DllLoadError::DlOpenError(error) => write!(f, "failed to open DLL: {}", error),
            DllLoadError::DllLayoutError(error) => write!(f, "{}", error),
            DllLoadError::UndefinedSymbol(name) => write!(f, "undefined symbol {}", name),
        }
    }
}

impl Error for DllLoadError {}

impl From<dlopen::Error> for DllLoadError {
    fn from(v: dlopen::Error) -> Self {
        Self::DlOpenError(v)
    }
}

impl From<DllLayoutError> for DllLoadError {
    fn from(v: DllLayoutError) -> Self {
        Self::DllLayoutError(v)
    }
}
