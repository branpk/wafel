use std::{error::Error, fmt};

use wafel_data_type::{TypeName, ValueError};
use wafel_layout::DllLayoutError;

#[derive(Debug, Clone)]
pub enum MemoryError {
    Context {
        context: String,
        error: Box<MemoryError>,
    },
    ValueError(ValueError),
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
            MemoryError::Context { context, error } => write!(f, "{}: {}", context, error),
            MemoryError::ValueError(error) => write!(f, "{}", error),
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

impl From<ValueError> for MemoryError {
    fn from(v: ValueError) -> Self {
        Self::ValueError(v)
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
