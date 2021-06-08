#![allow(missing_docs)]

use std::{error::Error, fmt};

use wafel_data_type::DataTypeRef;
use wafel_layout::LayoutLookupError;

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

#[derive(Debug, Clone)]
pub enum DataPathCompileError {
    LayoutLookupError(LayoutLookupError),
    ParseError { message: String },
    UndefinedSymbol { name: String },
    UndefinedField { name: String },
    NotAStruct { field_name: String },
    NotAnArray,
    IndexOutOfBounds { index: usize, length: usize },
    NullableNotAPointer,
    UnsizedBaseType,
}

impl fmt::Display for DataPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataPathError::CompileError { source, error } => {
                write!(f, "while compiling '{}': {}", source, error)
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

impl fmt::Display for DataPathCompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataPathCompileError::LayoutLookupError(error) => write!(f, "{}", error),
            DataPathCompileError::ParseError { message } => write!(f, "parse error: {}", message),
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
        }
    }
}

impl Error for DataPathCompileError {}

impl From<LayoutLookupError> for DataPathCompileError {
    fn from(v: LayoutLookupError) -> Self {
        Self::LayoutLookupError(v)
    }
}
