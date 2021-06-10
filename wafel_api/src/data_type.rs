use std::fmt;

use wafel_data_type::{DataTypeRef, FloatType, IntType};
use wafel_layout::DataLayout;

use crate::Error;

/// A simplified description of a variable's data type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    /// Void, typically used as a pointer target or function return type.
    Void,
    /// An integer type.
    Int(IntType),
    /// A float type.
    Float(FloatType),
    /// A pointer type.
    Pointer,
    /// An array type.
    Array,
    /// A struct type.
    Struct,
    /// A union type.
    Union,
}

impl DataType {
    /// Return true if the data type is void.
    pub fn is_void(&self) -> bool {
        matches!(self, Self::Void)
    }

    /// Return true if the data type is an integer type.
    pub fn is_int(&self) -> bool {
        matches!(self, Self::Int(_))
    }

    /// Return true if the data type is a float type.
    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(_))
    }

    /// Return true if the data type is a pointer type.
    pub fn is_pointer(&self) -> bool {
        matches!(self, Self::Pointer)
    }

    /// Return true if the data type is an array type.
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array)
    }

    /// Return true if the data type is a struct type.
    pub fn is_struct(&self) -> bool {
        matches!(self, Self::Struct)
    }

    /// Return true if the data type is a union type.
    pub fn is_union(&self) -> bool {
        matches!(self, Self::Union)
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataType::Void => write!(f, "void"),
            DataType::Int(int_type) => write!(f, "{}", int_type),
            DataType::Float(float_type) => write!(f, "{}", float_type),
            DataType::Pointer => write!(f, "pointer"),
            DataType::Array => write!(f, "array"),
            DataType::Struct => write!(f, "struct"),
            DataType::Union => write!(f, "union"),
        }
    }
}

pub(crate) fn simplified_data_type(
    layout: &DataLayout,
    data_type: &DataTypeRef,
) -> Result<DataType, Error> {
    use wafel_data_type::DataType::*;
    Ok(match data_type.as_ref() {
        Void => DataType::Void,
        Int(int_type) => DataType::Int(*int_type),
        Float(float_type) => DataType::Float(*float_type),
        Pointer { .. } => DataType::Pointer,
        Array { .. } => DataType::Array,
        Struct { .. } => DataType::Union,
        Union { .. } => DataType::Struct,
        Name(type_name) => simplified_data_type(layout, layout.data_type(type_name)?)?,
    })
}
