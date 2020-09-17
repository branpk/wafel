#![allow(missing_docs)]

use crate::memory::data_type::DataTypeRef;
use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
pub enum DataPathErrorCause {
    #[display(fmt = "parse error: {}", message)]
    ParseError { message: String },
    #[display(fmt = "undefined field {}", name)]
    UndefinedField { name: String },
    #[display(fmt = "accessing {} in non-struct type", field_name)]
    NotAStruct { field_name: String },
    #[display(fmt = "indexing into non-array type")]
    NotAnArray,
    #[display(fmt = "out of bounds: index {} in array of length {}", index, length)]
    IndexOutOfBounds { index: usize, length: usize },
    #[display(fmt = "nullable ? operator can only be used on a pointer")]
    NullableNotAPointer,
    #[display(fmt = "indexing through pointer with unsized base type")]
    UnsizedBaseType,
    #[display(
        fmt = "cannot concatenate path {} of type {} and path {} of type {}",
        path1,
        type1,
        path2,
        type2
    )]
    DataPathConcatTypeMismatch {
        path1: String,
        type1: DataTypeRef,
        path2: String,
        type2: DataTypeRef,
    },
    #[display(fmt = "expected global path, found {}", path)]
    ExpectedGlobalPath { path: String },
    #[display(fmt = "expected local path, found {}", path)]
    ExpectedLocalPath { path: String },
    #[display(fmt = "not a struct field: {}", path)]
    NotAField { path: String },
}
