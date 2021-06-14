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
    ReadUnsizedArray,
    ReadUnion,
    WriteExtraField(String),
    WriteMissingField(String),
    WriteUnion,
    InvalidAddress,
    WriteToStaticAddress,
    NonStaticAddressInStaticView,
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::Context { context, error } => write!(f, "{}:\n  {}", context, error),
            MemoryError::ValueTypeError(error) => write!(f, "{}", error),
            MemoryError::UndefinedTypeName(type_name) => {
                write!(f, "undefined type name: {}", type_name)
            }
            MemoryError::ReadUnsizedArray => {
                write!(f, "cannot read array with unknown length")
            }
            MemoryError::ReadUnion => {
                write!(f, "cannot read union with unspecified variant")
            }
            MemoryError::WriteExtraField(name) => {
                write!(f, "extra field when writing to struct: {}", name)
            }
            MemoryError::WriteMissingField(name) => {
                write!(f, "missing field when writing to struct: {}", name)
            }
            MemoryError::WriteUnion => {
                write!(f, "cannot write to union with unspecified variant")
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

impl From<ValueTypeError> for MemoryError {
    fn from(v: ValueTypeError) -> Self {
        Self::ValueTypeError(v)
    }
}

#[derive(Debug, Clone)]
pub enum DllLoadError {
    DlOpenError(Arc<dlopen::Error>),
    IoError(Arc<io::Error>),
    DllLayoutError(DllLayoutError),
    MissingDataSegments,
    UndefinedSymbol(String),
}

impl fmt::Display for DllLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DllLoadError::DlOpenError(error) => write!(f, "{}", error),
            DllLoadError::IoError(error) => write!(f, "{}", error),
            DllLoadError::DllLayoutError(error) => write!(f, "{}", error),
            DllLoadError::MissingDataSegments => write!(f, "missing data sections .data/.bss"),
            DllLoadError::UndefinedSymbol(name) => write!(f, "undefined symbol {}", name),
        }
    }
}

impl Error for DllLoadError {}

impl From<dlopen::Error> for DllLoadError {
    fn from(v: dlopen::Error) -> Self {
        Self::DlOpenError(Arc::new(v))
    }
}

impl From<io::Error> for DllLoadError {
    fn from(v: io::Error) -> Self {
        Self::IoError(Arc::new(v))
    }
}

impl From<DllLayoutError> for DllLoadError {
    fn from(v: DllLayoutError) -> Self {
        Self::DllLayoutError(v)
    }
}
