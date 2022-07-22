#![allow(missing_docs)]

use std::{error::Error, fmt, io, sync::Arc};

use wafel_data_type::{TypeName, ValueTypeError};
use wafel_layout::DllLayoutError;

#[derive(Debug, Clone)]
pub enum MemoryError {
    Context {
        context: String,
        error: Box<MemoryError>,
    },
    ValueTypeError(ValueTypeError),
    UndefinedTypeName(TypeName),
    UndefinedSymbol(String),
    InvalidAddress,
    WriteToStaticAddress,
    NonStaticAddressInStaticView,
    ProcessReadError(Arc<io::Error>),
    ProcessWriteError(Arc<io::Error>),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::Context { context, error } => write!(f, "{}:\n  {}", context, error),
            MemoryError::ValueTypeError(error) => write!(f, "{}", error),
            MemoryError::UndefinedTypeName(type_name) => {
                write!(f, "undefined type name: {}", type_name)
            }
            MemoryError::UndefinedSymbol(name) => {
                write!(f, "undefined symbol: {}", name)
            }
            MemoryError::InvalidAddress => write!(f, "null or invalid address"),
            MemoryError::WriteToStaticAddress => write!(f, "write to static address"),
            MemoryError::NonStaticAddressInStaticView => {
                write!(f, "using a non-static address through a static memory view")
            }
            MemoryError::ProcessReadError(error) => {
                write!(f, "failed to read process memory: {}", error)
            }
            MemoryError::ProcessWriteError(error) => {
                write!(f, "failed to write process memory: {}", error)
            }
        }
    }
}

impl Error for MemoryError {}

impl From<ValueTypeError> for MemoryError {
    fn from(v: ValueTypeError) -> Self {
        Self::ValueTypeError(v)
    }
}

#[derive(Debug, Clone)]
pub enum MemoryInitError {
    DlOpenError(Arc<dlopen::Error>),
    IoError(Arc<io::Error>),
    DllLayoutError(DllLayoutError),
    MissingDataSegments,
    UndefinedSymbol(String),
    ProcessAttachError(Arc<io::Error>),
}

impl fmt::Display for MemoryInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryInitError::DlOpenError(error) => write!(f, "{}", error),
            MemoryInitError::IoError(error) => write!(f, "{}", error),
            MemoryInitError::DllLayoutError(error) => write!(f, "{}", error),
            MemoryInitError::MissingDataSegments => write!(f, "missing data sections .data/.bss"),
            MemoryInitError::UndefinedSymbol(name) => write!(f, "undefined symbol {}", name),
            MemoryInitError::ProcessAttachError(error) => {
                write!(f, "failed to attach to process: {}", error)
            }
        }
    }
}

impl Error for MemoryInitError {}

impl From<dlopen::Error> for MemoryInitError {
    fn from(v: dlopen::Error) -> Self {
        Self::DlOpenError(Arc::new(v))
    }
}

impl From<io::Error> for MemoryInitError {
    fn from(v: io::Error) -> Self {
        Self::IoError(Arc::new(v))
    }
}

impl From<DllLayoutError> for MemoryInitError {
    fn from(v: DllLayoutError) -> Self {
        Self::DllLayoutError(v)
    }
}
