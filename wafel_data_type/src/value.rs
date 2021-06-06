//! Dynamically typed value used for reading and writing to memory.

use std::{borrow::Cow, collections::HashMap, convert::TryFrom, error::Error, fmt, ops::Add};

use serde::{Deserialize, Serialize};

/// A raw pointer value that can be stored in memory.
///
/// Having a single numeric type is convenient so that `Value` doesn't have to be generic
/// on a `Memory` implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address(pub usize);

impl Add<usize> for Address {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0.wrapping_add(rhs))
    }
}

impl Address {
    pub fn is_null(self) -> bool {
        self.0 == 0
    }
}

/// A dynamically typed value.
#[derive(Debug, Clone)]
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
    String(String),
    /// An address value.
    Address(Address),
    /// A struct value.
    Struct {
        /// The fields of a struct.
        ///
        /// If a field's name is present in the original struct definition, it will match the
        /// name used here. Anonymous fields will be given a name, typically `__anon`.
        fields: Box<HashMap<String, Value>>,
    },
    /// An array value.
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

impl Value {
    /// Return true if the value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Convert the value to an int.
    pub fn as_int(&self) -> Result<IntValue, ValueError> {
        if let Value::Int(n) = *self {
            Ok(n)
        } else {
            Err(ValueError::WrongType {
                expected: "int".into(),
                actual: self.clone(),
            })
        }
    }

    /// Convert the value to a usize.
    pub fn as_usize(&self) -> Result<usize, ValueError> {
        self.as_int().and_then(|n| {
            usize::try_from(n).map_err(|_| ValueError::WrongType {
                expected: "usize".into(),
                actual: self.clone(),
            })
        })
    }

    /// Convert the value to a float.
    pub fn as_float(&self) -> Result<FloatValue, ValueError> {
        if let Value::Float(r) = *self {
            Ok(r)
        } else {
            Err(ValueError::WrongType {
                expected: "float".into(),
                actual: self.clone(),
            })
        }
    }

    /// Convert the value to a float and then truncate to an f32.
    pub fn as_f32(&self) -> Result<f32, ValueError> {
        self.as_float().map(|r| r as f32)
    }

    /// Convert the value to an address.
    pub fn as_address(&self) -> Result<Address, ValueError> {
        if let Value::Address(address) = self {
            Ok(*address)
        } else {
            Err(ValueError::WrongType {
                expected: "address".into(),
                actual: self.clone(),
            })
        }
    }

    /// Convert the value to a struct and return its fields.
    pub fn as_struct(&self) -> Result<&HashMap<String, Value>, ValueError> {
        if let Value::Struct { fields } = self {
            Ok(fields)
        } else {
            Err(ValueError::WrongType {
                expected: "struct".into(),
                actual: self.clone(),
            })
        }
    }

    /// Convert the value to an array and return its elements.
    pub fn as_array(&self) -> Result<&[Value], ValueError> {
        if let Value::Array(elements) = self {
            Ok(elements)
        } else {
            Err(ValueError::WrongType {
                expected: "array".into(),
                actual: self.clone(),
            })
        }
    }

    /// Convert the value to an array and return its elements.
    pub fn as_array_with_len(&self, length: usize) -> Result<&[Value], ValueError> {
        let elements = self.as_array()?;
        if elements.len() == length {
            Ok(elements)
        } else {
            Err(ValueError::WrongType {
                expected: format!("array of length {}", length).into(),
                actual: self.clone(),
            })
        }
    }

    /// Convert the value to an array of three i16s.
    pub fn as_i16_3(&self) -> Result<[i16; 3], ValueError> {
        let elements = self.as_array_with_len(3)?;
        Ok([
            elements[0].as_int()? as i16,
            elements[1].as_int()? as i16,
            elements[2].as_int()? as i16,
        ])
    }

    /// Convert the value to an array of three f32s.
    pub fn as_f32_3(&self) -> Result<[f32; 3], ValueError> {
        let elements = self.as_array_with_len(3)?;
        Ok([
            elements[0].as_float()? as f32,
            elements[1].as_float()? as f32,
            elements[2].as_float()? as f32,
        ])
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#X}", self.0)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(r) => write!(f, "{}", r),
            Value::String(s) => write!(f, "{:?}", s),
            Value::Address(a) => write!(f, "{}", a),
            Value::Struct { fields } => {
                write!(
                    f,
                    "{{ {} }}",
                    fields
                        .iter()
                        .map(|(name, value)| format!("{} = {}", name, value))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Value::Array(elements) => {
                write!(
                    f,
                    "[{}]",
                    elements
                        .iter()
                        .map(|element| format!("{}", element))
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValueError {
    WrongType {
        expected: Cow<'static, str>,
        actual: Value,
    },
}

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueError::WrongType { expected, actual } => {
                write!(f, "expected value of type: {}, found: {}", expected, actual)
            }
        }
    }
}

impl Error for ValueError {}
