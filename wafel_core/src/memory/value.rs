//! Dynamically typed value used for reading and writing to memory.

use super::{Address, MemoryErrorCause};
use crate::error::Error;
use derive_more::Display;
use std::{collections::HashMap, convert::TryFrom};

/// A dynamically typed value.
#[derive(Debug, Display, Clone)]
pub enum Value {
    /// Represents a null value.
    ///
    /// When evaluating a data path, this only occurs when `?` is used on an invalid
    /// pointer. Otherwise, `Value::Address` will be used (or an error will be thrown
    /// when it is dereferenced).
    Null,
    /// An integer value, regardless of the underlying `IntType` size.
    Int(IntValue),
    /// A float value, regardless of the underlying `FloatType` size.
    Float(FloatValue),
    /// A string value.
    #[display(fmt = "{:?}", _0)]
    String(String),
    /// An address value.
    Address(Address),
    /// A struct value.
    #[display(fmt = "{}", "display_struct(fields)")]
    Struct {
        /// The fields of a struct.
        ///
        /// If a field's name is present in the original struct definition, it will match the
        /// name used here. Anonymous fields will be given a name, typically `__anon`.
        fields: Box<HashMap<String, Value>>,
    },
    /// An array value.
    #[display(fmt = "{}", "display_array(_0)")]
    Array(Vec<Value>),
}

/// An integer value.
///
/// i128 is used so that any `IntType` can fit in it.
pub type IntValue = i128;

/// An integer value.
///
/// f64 is used so that any `FloatType` can fit in it.
pub type FloatValue = f64;

fn display_struct(fields: &HashMap<String, Value>) -> String {
    let field_str = fields
        .iter()
        .map(|(name, value)| format!("{} = {}", name, value))
        .collect::<Vec<String>>()
        .join(", ");
    format!("{{ {} }}", field_str)
}

fn display_array(elements: &[Value]) -> String {
    let elements_str = elements
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join(", ");
    format!("[{}]", elements_str)
}

impl Value {
    /// Return true if the value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Convert the value to an int.
    //
    /// Return an error if the value is not an int.
    pub fn as_int(&self) -> Result<IntValue, Error> {
        if let Value::Int(n) = *self {
            Ok(n)
        } else {
            Err(MemoryErrorCause::ValueTypeError {
                value: self.to_string(),
                expected: "int".to_owned(),
            }
            .into())
        }
    }

    /// Convert the value to a usize.
    ///
    /// Return an error if the value is not an int or the value is out of range.
    pub fn as_usize(&self) -> Result<usize, Error> {
        let value = self.as_int()?;
        usize::try_from(value).map_err(|_| {
            MemoryErrorCause::ValueTypeError {
                value: self.to_string(),
                expected: "usize".to_owned(),
            }
            .into()
        })
    }

    /// Convert the value to a float.
    //
    /// Return an error if the value is not a float.
    pub fn as_float(&self) -> Result<FloatValue, Error> {
        if let Value::Float(r) = *self {
            Ok(r)
        } else {
            Err(MemoryErrorCause::ValueTypeError {
                value: self.to_string(),
                expected: "float".to_owned(),
            }
            .into())
        }
    }

    /// Convert the value to a float and then truncate to an f32.
    //
    /// Return an error if the value is not a float.
    pub fn as_f32(&self) -> Result<f32, Error> {
        Ok(self.as_float()? as f32)
    }

    /// Convert the value to an address.
    //
    /// Return an error if the value is not an address.
    pub fn as_address(&self) -> Result<Address, Error> {
        if let Value::Address(address) = self {
            Ok(*address)
        } else {
            Err(MemoryErrorCause::ValueTypeError {
                value: self.to_string(),
                expected: "address".to_owned(),
            }
            .into())
        }
    }

    /// Convert the value to a struct and return its fields.
    //
    /// Return an error if the value is not an address.
    pub fn as_struct(&self) -> Result<&HashMap<String, Value>, Error> {
        if let Value::Struct { fields } = self {
            Ok(fields)
        } else {
            Err(MemoryErrorCause::ValueTypeError {
                value: self.to_string(),
                expected: "struct".to_owned(),
            }
            .into())
        }
    }

    /// Convert the value to an array and return its elements.
    ///
    /// Return an error if the value is not an array.
    pub fn as_array(&self) -> Result<&[Value], Error> {
        if let Value::Array(elements) = self {
            Ok(elements)
        } else {
            Err(MemoryErrorCause::ValueTypeError {
                value: self.to_string(),
                expected: "array".to_owned(),
            }
            .into())
        }
    }

    /// Convert the value to an array and return its elements.
    ///
    /// Return an error if the value is not an array of the specified length.
    pub fn as_array_with_len(&self, length: usize) -> Result<&[Value], Error> {
        let elements = self.as_array()?;
        if elements.len() == length {
            Ok(elements)
        } else {
            Err(MemoryErrorCause::ValueTypeError {
                value: self.to_string(),
                expected: format!("array of length {}", length),
            }
            .into())
        }
    }

    /// Convert the value to an array of three i16s.
    ///
    /// Return an error if the value is not an array of three ints.
    pub fn as_i16_3(&self) -> Result<[i16; 3], Error> {
        let elements = self.as_array_with_len(3)?;
        Ok([
            elements[0].as_int()? as i16,
            elements[1].as_int()? as i16,
            elements[2].as_int()? as i16,
        ])
    }

    /// Convert the value to an array of three f32s.
    ///
    /// Return an error if the value is not an array of three floats.
    pub fn as_f32_3(&self) -> Result<[f32; 3], Error> {
        let elements = self.as_array_with_len(3)?;
        Ok([
            elements[0].as_float()? as f32,
            elements[1].as_float()? as f32,
            elements[2].as_float()? as f32,
        ])
    }
}
