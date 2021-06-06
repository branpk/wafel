#![allow(missing_docs)]

use crate::error::WithContext;
use derive_more::{Display, Error, From};
use std::io;
use wafel_data_type::shallow::BuildDataTypesError;

pub type DllError = WithContext<DllErrorCause>;

#[derive(Debug, Display, Error, From)]
pub enum DllErrorCause {
    #[from]
    DlOpenError(dlopen::Error),
    #[from]
    LayoutError(LayoutErrorCause),
    #[display(fmt = "while reading {}: {}", name, source)]
    SymbolReadError { name: String, source: dlopen::Error },
    #[display(fmt = "empty data layout when loading DLL (no DWARF info?)")]
    EmptyDataLayout,
    #[display(fmt = "missing segment {}", name)]
    MissingSegment { name: String },
    #[display(fmt = "overlapping DLL segments: {} and {}", name1, name2)]
    OverlappingSegments { name1: String, name2: String },
}

pub type LayoutError = WithContext<LayoutErrorCause>;

#[derive(Debug, Display, Error, From)]
pub enum LayoutErrorCause {
    #[display(fmt = "file error: {}", _0)]
    #[from]
    FileReadError(io::Error),
    #[display(fmt = "parse error: {}", _0)]
    #[from]
    ObjectReadError(object::Error),
    #[display(fmt = "dwarf error: {}", _0)]
    #[from]
    DwarfReadError(gimli::Error),
    #[from]
    BuildDataTypesError(BuildDataTypesError<String>),
    #[display(fmt = "missing attribute {} in entry {}", attribute, entry_label)]
    MissingAttribute {
        entry_label: String,
        attribute: gimli::DwAt,
    },
    #[display(fmt = "expected dwarf tag {}, found {}", expected, actual)]
    UnexpectedTag {
        entry_label: String,
        expected: gimli::DwTag,
        actual: gimli::DwTag,
    },
    #[display(fmt = "unimplemented base type name {}", name)]
    UnknownBaseTypeName { name: String },
    #[display(fmt = "expected subrange node")]
    MissingSubrangeNode { entry_label: String },
}

impl From<LayoutError> for DllError {
    fn from(error: LayoutError) -> Self {
        error.cause_into()
    }
}

impl From<dlopen::Error> for DllError {
    fn from(error: dlopen::Error) -> Self {
        DllErrorCause::from(error).into()
    }
}

impl From<io::Error> for LayoutError {
    fn from(error: io::Error) -> Self {
        LayoutErrorCause::from(error).into()
    }
}

impl From<object::Error> for LayoutError {
    fn from(error: object::Error) -> Self {
        LayoutErrorCause::from(error).into()
    }
}

impl From<gimli::Error> for LayoutError {
    fn from(error: gimli::Error) -> Self {
        LayoutErrorCause::from(error).into()
    }
}

impl From<BuildDataTypesError<String>> for LayoutError {
    fn from(error: BuildDataTypesError<String>) -> Self {
        LayoutErrorCause::from(error).into()
    }
}
