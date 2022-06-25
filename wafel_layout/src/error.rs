#![allow(missing_docs)]

use std::{error::Error, fmt, io, sync::Arc};

use wafel_data_type::{shallow::BuildDataTypesError, TypeName};

#[derive(Debug, Clone)]
pub struct DllLayoutError {
    pub kind: DllLayoutErrorKind,
    pub unit: Option<String>,
}

impl fmt::Display for DllLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.unit {
            Some(unit) => write!(f, "in unit {}:\n  {}", unit, self.kind),
            None => write!(f, "{}", self.kind),
        }
    }
}

impl Error for DllLayoutError {}

#[derive(Debug, Clone)]
pub enum DllLayoutErrorKind {
    FileReadError(Arc<io::Error>),
    ObjectReadError(object::Error),
    DwarfReadError(gimli::Error),
    NoDwarfInfo,
    BuildDataTypesError(BuildDataTypesError<String>),
    MissingAttribute {
        entry_label: String,
        attribute: gimli::DwAt,
    },
    UnexpectedTag {
        entry_label: String,
        expected: gimli::DwTag,
        actual: gimli::DwTag,
    },
    UnknownBaseTypeName {
        name: String,
    },
    MissingSubrangeNode {
        entry_label: String,
    },
    MissingDeclaration,
}

impl fmt::Display for DllLayoutErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DllLayoutErrorKind::FileReadError(error) => write!(f, "file error: {}", error),
            DllLayoutErrorKind::ObjectReadError(error) => write!(f, "object file error: {}", error),
            DllLayoutErrorKind::DwarfReadError(error) => write!(f, "dwarf error: {}", error),
            DllLayoutErrorKind::NoDwarfInfo => write!(f, "missing dwarf debug symbols"),
            DllLayoutErrorKind::BuildDataTypesError(error) => write!(f, "{}", error),
            DllLayoutErrorKind::MissingAttribute {
                entry_label,
                attribute,
            } => write!(
                f,
                "missing attribute {} in entry {}",
                attribute, entry_label
            ),
            DllLayoutErrorKind::UnexpectedTag {
                entry_label: _,
                expected,
                actual,
            } => write!(f, "expected dwarf tag {}, found {}", expected, actual),
            DllLayoutErrorKind::UnknownBaseTypeName { name } => {
                write!(f, "unimplemented base type name {}", name)
            }
            DllLayoutErrorKind::MissingSubrangeNode { entry_label: _ } => {
                write!(f, "expected subrange node")
            }
            DllLayoutErrorKind::MissingDeclaration => write!(f, "missing variable declaration"),
        }
    }
}

impl Error for DllLayoutErrorKind {}

impl From<io::Error> for DllLayoutError {
    fn from(v: io::Error) -> Self {
        DllLayoutError {
            kind: DllLayoutErrorKind::from(v),
            unit: None,
        }
    }
}

impl From<object::Error> for DllLayoutError {
    fn from(v: object::Error) -> Self {
        DllLayoutError {
            kind: DllLayoutErrorKind::from(v),
            unit: None,
        }
    }
}

impl From<gimli::Error> for DllLayoutError {
    fn from(v: gimli::Error) -> Self {
        DllLayoutError {
            kind: DllLayoutErrorKind::from(v),
            unit: None,
        }
    }
}

impl From<io::Error> for DllLayoutErrorKind {
    fn from(v: io::Error) -> Self {
        Self::FileReadError(Arc::new(v))
    }
}

impl From<object::Error> for DllLayoutErrorKind {
    fn from(v: object::Error) -> Self {
        Self::ObjectReadError(v)
    }
}

impl From<gimli::Error> for DllLayoutErrorKind {
    fn from(v: gimli::Error) -> Self {
        Self::DwarfReadError(v)
    }
}

impl From<BuildDataTypesError<String>> for DllLayoutErrorKind {
    fn from(v: BuildDataTypesError<String>) -> Self {
        Self::BuildDataTypesError(v)
    }
}

#[derive(Debug, Clone)]
pub enum LayoutLookupError {
    UndefinedTypeName(TypeName),
    UndefinedGlobal(String),
    UndefinedConstant(String),
}

impl fmt::Display for LayoutLookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayoutLookupError::UndefinedTypeName(type_name) => {
                write!(f, "undefined type name: {}", type_name)
            }
            LayoutLookupError::UndefinedGlobal(name) => {
                write!(f, "undefined global name: {}", name)
            }
            LayoutLookupError::UndefinedConstant(name) => {
                write!(f, "undefined constant name: {}", name)
            }
        }
    }
}

impl Error for LayoutLookupError {}

#[derive(Debug, Clone)]
pub enum SM64LayoutError {
    LayoutLookupError(LayoutLookupError),
    ObjectStructInUse,
    ObjectStructNotStruct,
    MissingRawData,
    RawDataNotUnion,
    MissingRawDataArray(String),
    InvalidIndex(String),
    UnknownVersion(String),
}

impl fmt::Display for SM64LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SM64LayoutError::LayoutLookupError(error) => write!(f, "{}", error),
            SM64LayoutError::ObjectStructInUse => write!(f, "Object type already in use"),
            SM64LayoutError::ObjectStructNotStruct => write!(f, "Object type is not a struct"),
            SM64LayoutError::MissingRawData => write!(f, "missing rawData field in struct Object"),
            SM64LayoutError::RawDataNotUnion => {
                write!(f, "struct Object rawData field is not a union")
            }
            SM64LayoutError::MissingRawDataArray(array) => {
                write!(f, "missing rawData.{} array in struct Object", array)
            }
            SM64LayoutError::InvalidIndex(name) => {
                write!(f, "invalid index in object field {}", name)
            }
            SM64LayoutError::UnknownVersion(version) => {
                write!(
                    f,
                    "unknown SM64 version: {} (available: us, jp, eu, sh)",
                    version
                )
            }
        }
    }
}

impl Error for SM64LayoutError {}

impl From<LayoutLookupError> for SM64LayoutError {
    fn from(v: LayoutLookupError) -> Self {
        Self::LayoutLookupError(v)
    }
}
