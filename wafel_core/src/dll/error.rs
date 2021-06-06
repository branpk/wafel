#![allow(missing_docs)]

use crate::error::WithContext;
use derive_more::{Display, Error, From};
use wafel_layout::DllLayoutError;

pub type DllError = WithContext<DllErrorCause>;

#[derive(Debug, Display, Error, From)]
pub enum DllErrorCause {
    #[from]
    DlOpenError(dlopen::Error),
    #[from]
    LayoutError(DllLayoutError),
    #[display(fmt = "while reading {}: {}", name, source)]
    SymbolReadError { name: String, source: dlopen::Error },
    #[display(fmt = "empty data layout when loading DLL (no DWARF info?)")]
    EmptyDataLayout,
    #[display(fmt = "missing segment {}", name)]
    MissingSegment { name: String },
    #[display(fmt = "overlapping DLL segments: {} and {}", name1, name2)]
    OverlappingSegments { name1: String, name2: String },
}

impl From<dlopen::Error> for DllError {
    fn from(error: dlopen::Error) -> Self {
        DllErrorCause::from(error).into()
    }
}
