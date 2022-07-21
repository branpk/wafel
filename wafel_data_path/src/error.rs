#![allow(missing_docs)]

use std::{error::Error, fmt};

use wafel_data_type::{DataTypeError, DataTypeRef};
use wafel_layout::LayoutLookupError;
use wafel_memory::MemoryError;

#[derive(Debug, Clone)]
pub enum DataError {
    DataPathError(DataPathError),
    MemoryError(MemoryError),
    LayoutLookupError(LayoutLookupError),
    DataTypeError(DataTypeError),
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataError::DataPathError(error) => write!(f, "{}", error),
            DataError::MemoryError(error) => write!(f, "{}", error),
            DataError::LayoutLookupError(error) => write!(f, "{}", error),
            DataError::DataTypeError(error) => write!(f, "{}", error),
        }
    }
}

impl Error for DataError {}

impl From<DataPathError> for DataError {
    fn from(v: DataPathError) -> Self {
        Self::DataPathError(v)
    }
}

impl From<MemoryError> for DataError {
    fn from(v: MemoryError) -> Self {
        Self::MemoryError(v)
    }
}

impl From<LayoutLookupError> for DataError {
    fn from(v: LayoutLookupError) -> Self {
        Self::LayoutLookupError(v)
    }
}

impl From<DataTypeError> for DataError {
    fn from(v: DataTypeError) -> Self {
        Self::DataTypeError(v)
    }
}

#[derive(Debug, Clone)]
pub enum DataPathError {
    CompileError {
        source: String,
        error: DataPathCompileError,
    },
    ConcatTypeMismatch {
        path1: String,
        type1: DataTypeRef,
        path2: String,
        type2: DataTypeRef,
    },
    ExpectedGlobalPath {
        path: String,
    },
    ExpectedLocalPath {
        path: String,
    },
    NotAField {
        path: String,
    },
}

impl fmt::Display for DataPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataPathError::CompileError { source, error } => {
                write!(f, "while compiling '{}':\n  {}", source, error)
            }
            DataPathError::ConcatTypeMismatch {
                path1,
                type1,
                path2,
                type2,
            } => write!(
                f,
                "cannot concatenate path {} of type {} and path {} of type {}",
                path1, type1, path2, type2
            ),
            DataPathError::ExpectedGlobalPath { path } => {
                write!(f, "expected global path, found {}", path)
            }
            DataPathError::ExpectedLocalPath { path } => {
                write!(f, "expected local path, found {}", path)
            }
            DataPathError::NotAField { path } => write!(f, "not a struct field: {}", path),
        }
    }
}

impl Error for DataPathError {}

#[derive(Debug, Clone)]
pub enum DataPathCompileError {
    ParseError(String),
    LayoutLookupError(LayoutLookupError),
    UndefinedSymbol { name: String },
    UndefinedField { name: String },
    NotAStruct { field_name: String },
    NotAnArray,
    IndexOutOfBounds { index: usize, length: usize },
    NullableNotAPointer,
    UnsizedBaseType,
    MaskOnNonInt,
}

impl fmt::Display for DataPathCompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataPathCompileError::ParseError(message) => {
                write!(f, "syntax error: {}", message)
            }
            DataPathCompileError::LayoutLookupError(error) => write!(f, "{}", error),
            DataPathCompileError::UndefinedField { name } => write!(f, "undefined field {}", name),
            DataPathCompileError::UndefinedSymbol { name } => {
                write!(f, "undefined symbol {}", name)
            }
            DataPathCompileError::NotAStruct { field_name } => {
                write!(f, "accessing {} in non-struct type", field_name)
            }
            DataPathCompileError::NotAnArray => write!(f, "indexing into non-array type"),
            DataPathCompileError::IndexOutOfBounds { index, length } => write!(
                f,
                "out of bounds: index {} in array of length {}",
                index, length
            ),
            DataPathCompileError::NullableNotAPointer => {
                write!(f, "nullable ? operator can only be used on a pointer")
            }
            DataPathCompileError::UnsizedBaseType => {
                write!(f, "indexing through pointer with unsized base type")
            }
            DataPathCompileError::MaskOnNonInt => {
                write!(f, "mask applied to non-integer variable")
            }
        }
    }
}

impl Error for DataPathCompileError {}

impl From<LayoutLookupError> for DataPathCompileError {
    fn from(v: LayoutLookupError) -> Self {
        Self::LayoutLookupError(v)
    }
}
