#![allow(missing_docs)]

use std::{error, fmt, io, sync::Arc};

use wafel_data_path::{DataPathError, GlobalDataPath};
use wafel_data_type::{IntValue, Value, ValueTypeError};
use wafel_layout::{DllLayoutError, LayoutLookupError, SM64ExtrasError};
use wafel_memory::{DllLoadError, MemoryError};

#[derive(Debug, Clone)]
pub enum Error {
    DllLayoutError(DllLayoutError),
    SM64ExtrasError(SM64ExtrasError),
    DllLoadError(DllLoadError),
    DataPathError(DataPathError),
    MemoryError(MemoryError),
    ApplyEditError {
        path: Arc<GlobalDataPath>,
        value: Value,
        error: MemoryError,
    },
    LayoutLookupError(LayoutLookupError),
    SaveStateMismatch,
    ValueTypeError(ValueTypeError),
    InvalidFrameLogEventType(IntValue),
    UnsizedSurfacePoolPointer,
    UnsizedObjectPoolArray,
    M64ReadError {
        filename: String,
        error: Arc<io::Error>,
    },
    InvalidM64Error {
        filename: String,
    },
    M64WriteError {
        filename: String,
        error: Arc<io::Error>,
    },
    M64AuthorTooLong,
    M64DescriptionTooLong,
    FileReadError {
        filename: String,
        error: Arc<io::Error>,
    },
    FileWriteError {
        filename: String,
        error: Arc<io::Error>,
    },
    Libsm64EncryptionError,
    Libsm64DecryptionError,
    InvalidRom,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::DllLayoutError(error) => write!(f, "{}", error),
            Error::SM64ExtrasError(error) => write!(f, "{}", error),
            Error::DllLoadError(error) => write!(f, "{}", error),
            Error::DataPathError(error) => write!(f, "{}", error),
            Error::MemoryError(error) => write!(f, "{}", error),
            Error::ApplyEditError { path, value, error } => {
                write!(f, "while applying edit {} = {}:\n  {}", path, value, error)
            }
            Error::LayoutLookupError(error) => write!(f, "{}", error),
            Error::SaveStateMismatch => {
                write!(f, "save state was created by a different Game instance")
            }
            Error::ValueTypeError(error) => write!(f, "{}", error),
            Error::InvalidFrameLogEventType(value) => {
                write!(f, "invalid frame log event type: {}", value)
            }
            Error::UnsizedSurfacePoolPointer => {
                write!(f, "surface pool array does not have a stride")
            }
            Error::UnsizedObjectPoolArray => {
                write!(f, "object pool array does not have a stride")
            }
            Error::M64ReadError { filename, error } => {
                write!(f, "failed to read {}:\n  {}", filename, error)
            }
            Error::InvalidM64Error { filename } => {
                write!(f, "invalid .m64 file: {}", filename)
            }
            Error::M64WriteError { filename, error } => {
                write!(f, "failed to write {}:\n  {}", filename, error)
            }
            Error::M64AuthorTooLong => write!(f, "author field too long (max 222 bytes)"),
            Error::M64DescriptionTooLong => write!(f, "description field too long (max 256 bytes)"),
            Error::FileReadError { filename, error } => {
                write!(f, "failed to read {}:\n  {}", filename, error)
            }
            Error::FileWriteError { filename, error } => {
                write!(f, "failed to write {}:\n  {}", filename, error)
            }
            Error::Libsm64EncryptionError => write!(f, "failed to encrypt libsm64"),
            Error::Libsm64DecryptionError => write!(
                f,
                "failed to decrypt libsm64. Are you using a vanilla ROM with the correct SM64 version?"
            ),
            Error::InvalidRom => write!(f, "provided file is not a valid SM64 ROM"),
        }
    }
}

impl error::Error for Error {}

impl From<DllLayoutError> for Error {
    fn from(v: DllLayoutError) -> Self {
        Self::DllLayoutError(v)
    }
}

impl From<SM64ExtrasError> for Error {
    fn from(v: SM64ExtrasError) -> Self {
        Self::SM64ExtrasError(v)
    }
}

impl From<DllLoadError> for Error {
    fn from(v: DllLoadError) -> Self {
        Self::DllLoadError(v)
    }
}

impl From<DataPathError> for Error {
    fn from(v: DataPathError) -> Self {
        Self::DataPathError(v)
    }
}

impl From<MemoryError> for Error {
    fn from(v: MemoryError) -> Self {
        Self::MemoryError(v)
    }
}

impl From<LayoutLookupError> for Error {
    fn from(v: LayoutLookupError) -> Self {
        Self::LayoutLookupError(v)
    }
}

impl From<ValueTypeError> for Error {
    fn from(v: ValueTypeError) -> Self {
        Self::ValueTypeError(v)
    }
}
