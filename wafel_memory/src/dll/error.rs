use std::{error::Error, fmt};

use wafel_layout::DllLayoutError;

#[derive(Debug)]
pub enum DllLoadError {
    DlOpenError(dlopen::Error),
    DllLayoutError(DllLayoutError),
    UndefinedSymbol(UndefinedSymbolError),
}

impl fmt::Display for DllLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
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

impl From<UndefinedSymbolError> for DllLoadError {
    fn from(v: UndefinedSymbolError) -> Self {
        Self::UndefinedSymbol(v)
    }
}

#[derive(Debug)]
pub struct UndefinedSymbolError {
    pub name: String,
    pub error: dlopen::Error,
}

impl fmt::Display for UndefinedSymbolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Error for UndefinedSymbolError {}
