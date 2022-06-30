//! Dynamically typed value used for reading and writing to memory.

use std::{
    array::IntoIter,
    collections::HashMap,
    convert::TryFrom,
    fmt,
    ops::{Add, AddAssign},
};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::ValueTypeError;

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

impl AddAssign<usize> for Address {
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

impl Address {
    /// Returns true if the address is null (equal to zero).
    pub fn is_null(self) -> bool {
        self.0 == 0
    }
}

/// A dynamically typed value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Represents the lack of a value.
    ///
    /// For example, when evaluating a data path and `?` is used on a null pointer,
    /// `Value::None` is returned for the entire path.
    None,
    /// An integer value, regardless of the underlying `IntType` size.
    Int(IntValue),
    /// A float value, regardless of the underlying `FloatType` size.
    Float(FloatValue),
    /// A string value.
    String(String),
    /// An address value.
    Address(Address),
    /// A struct value.
    ///
    /// If a field's name is present in the original struct definition, it will match the
    /// name used here. Anonymous fields will be given a name, typically `__anon`.
    Struct(Box<IndexMap<String, Value>>),
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
    /// Return true if the value is `Value::None`.
    pub fn is_none(&self) -> bool {
        matches!(self, Value::None)
    }

    /// Panic if the value is not `Value::None`.
    #[track_caller]
    pub fn as_none(&self) {
        if let Err(error) = self.try_as_none() {
            panic!("{}", error);
        }
    }

    /// Return an error if the value is not `Value::None`.
    pub fn try_as_none(&self) -> Result<(), ValueTypeError> {
        if self.is_none() {
            Ok(())
        } else {
            Err(ValueTypeError {
                expected: "void".into(),
                actual: self.clone(),
            })
        }
    }

    /// Return None if the value is none, and Some(self) otherwise.
    pub fn option(&self) -> Option<&Value> {
        if self.is_none() {
            None
        } else {
            Some(self)
        }
    }

    /// Convert the value to an int, panicking if it is not an int.
    #[track_caller]
    pub fn as_int(&self) -> IntValue {
        match self.try_as_int() {
            Ok(n) => n,
            Err(error) => panic!("{}", error),
        }
    }

    /// Convert the value to an int.
    pub fn try_as_int(&self) -> Result<IntValue, ValueTypeError> {
        if let Value::Int(n) = *self {
            Ok(n)
        } else {
            Err(ValueTypeError {
                expected: "int".into(),
                actual: self.clone(),
            })
        }
    }

    /// Convert the value to an int, allowing in-range floats that are integers.
    ///
    /// Panics if the conversion fails.
    #[track_caller]
    pub fn as_int_lenient(&self) -> IntValue {
        match self.try_as_int_lenient() {
            Ok(n) => n,
            Err(error) => panic!("{}", error),
        }
    }

    /// Convert the value to an int, allowing in-range floats that are integers.
    #[allow(clippy::float_cmp)]
    pub fn try_as_int_lenient(&self) -> Result<IntValue, ValueTypeError> {
        match *self {
            Value::Int(n) => Ok(n),
            Value::Float(r) if r as IntValue as FloatValue == r => Ok(r as IntValue),
            _ => Err(ValueTypeError {
                expected: "int".into(),
                actual: self.clone(),
            }),
        }
    }

    /// Convert the value to a usize, panicking on failure.
    #[track_caller]
    pub fn as_usize(&self) -> usize {
        match self.try_as_usize() {
            Ok(n) => n,
            Err(error) => panic!("{}", error),
        }
    }

    /// Convert the value to a usize.
    pub fn try_as_usize(&self) -> Result<usize, ValueTypeError> {
        self.try_as_int().and_then(|n| {
            usize::try_from(n).map_err(|_| ValueTypeError {
                expected: "usize".into(),
                actual: self.clone(),
            })
        })
    }

    /// Convert the value to a float, panicking if the value is not a float.
    #[track_caller]
    pub fn as_float(&self) -> FloatValue {
        match self.try_as_float() {
            Ok(r) => r,
            Err(error) => panic!("{}", error),
        }
    }

    /// Convert the value to a float.
    pub fn try_as_float(&self) -> Result<FloatValue, ValueTypeError> {
        if let Value::Float(r) = *self {
            Ok(r)
        } else {
            Err(ValueTypeError {
                expected: "float".into(),
                actual: self.clone(),
            })
        }
    }

    /// Convert the value to a usize, allowing in-range integers.
    ///
    /// Panics on failure.
    #[track_caller]
    pub fn as_float_lenient(&self) -> FloatValue {
        match self.try_as_float_lenient() {
            Ok(r) => r,
            Err(error) => panic!("{}", error),
        }
    }

    /// Convert the value to a float, allowing in-range integers.
    pub fn try_as_float_lenient(&self) -> Result<FloatValue, ValueTypeError> {
        match *self {
            Value::Float(r) => Ok(r),
            Value::Int(n) if n as FloatValue as IntValue == n => Ok(n as FloatValue),
            _ => Err(ValueTypeError {
                expected: "float".into(),
                actual: self.clone(),
            }),
        }
    }

    /// Convert the value to a float, and then truncate to an f32, panicking on failure.
    #[track_caller]
    pub fn as_f32(&self) -> f32 {
        match self.try_as_f32() {
            Ok(r) => r,
            Err(error) => panic!("{}", error),
        }
    }

    /// Convert the value to a float and then truncate to an f32.
    pub fn try_as_f32(&self) -> Result<f32, ValueTypeError> {
        self.try_as_float().map(|r| r as f32)
    }

    /// Convert the value to a string, panicking on failure.
    #[track_caller]
    pub fn as_str(&self) -> &str {
        match self.try_as_str() {
            Ok(s) => s,
            Err(error) => panic!("{}", error),
        }
    }

    /// Convert the value to a string.
    pub fn try_as_str(&self) -> Result<&str, ValueTypeError> {
        if let Value::String(s) = self {
            Ok(s)
        } else {
            Err(ValueTypeError {
                expected: "string".into(),
                actual: self.clone(),
            })
        }
    }

    /// Convert the value to an address, panicking on failure.
    #[track_caller]
    pub fn as_address(&self) -> Address {
        match self.try_as_address() {
            Ok(r) => r,
            Err(error) => panic!("{}", error),
        }
    }

    /// Convert the value to an address.
    pub fn try_as_address(&self) -> Result<Address, ValueTypeError> {
        if let Value::Address(address) = self {
            Ok(*address)
        } else {
            Err(ValueTypeError {
                expected: "address".into(),
                actual: self.clone(),
            })
        }
    }

    /// Convert the value to a struct and return its fields, panicking on failure.
    #[track_caller]
    pub fn as_struct(&self) -> &IndexMap<String, Value> {
        match self.try_as_struct() {
            Ok(fields) => fields,
            Err(error) => panic!("{}", error),
        }
    }

    /// Convert the value to a struct and return its fields.
    pub fn try_as_struct(&self) -> Result<&IndexMap<String, Value>, ValueTypeError> {
        if let Value::Struct(fields) = self {
            Ok(fields)
        } else {
            Err(ValueTypeError {
                expected: "struct".into(),
                actual: self.clone(),
            })
        }
    }

    /// Convert the value to an array and return its elements, panicking on failure.
    #[track_caller]
    pub fn as_array(&self) -> &[Value] {
        match self.try_as_array() {
            Ok(elements) => elements,
            Err(error) => panic!("{}", error),
        }
    }

    /// Convert the value to an array and return its elements.
    pub fn try_as_array(&self) -> Result<&[Value], ValueTypeError> {
        if let Value::Array(elements) = self {
            Ok(elements)
        } else {
            Err(ValueTypeError {
                expected: "array".into(),
                actual: self.clone(),
            })
        }
    }

    /// Convert the value to an array and return its elements, panicking on failure.
    #[track_caller]
    pub fn as_array_with_len(&self, length: usize) -> &[Value] {
        match self.try_as_array_with_len(length) {
            Ok(elements) => elements,
            Err(error) => panic!("{}", error),
        }
    }

    /// Convert the value to an array and return its elements.
    pub fn try_as_array_with_len(&self, length: usize) -> Result<&[Value], ValueTypeError> {
        let elements = self.try_as_array()?;
        if elements.len() == length {
            Ok(elements)
        } else {
            Err(ValueTypeError {
                expected: format!("array of length {}", length).into(),
                actual: self.clone(),
            })
        }
    }

    /// Convert the value to an array of three i16s, panicking on failure.
    #[track_caller]
    pub fn as_i16_3(&self) -> [i16; 3] {
        match self.try_as_i16_3() {
            Ok(elements) => elements,
            Err(error) => panic!("{}", error),
        }
    }

    /// Convert the value to an array of three i16s.
    pub fn try_as_i16_3(&self) -> Result<[i16; 3], ValueTypeError> {
        let elements = self.try_as_array_with_len(3)?;
        Ok([
            elements[0].try_as_int()? as i16,
            elements[1].try_as_int()? as i16,
            elements[2].try_as_int()? as i16,
        ])
    }

    /// Convert the value to an array of three f32s, panicking on failure.
    #[track_caller]
    pub fn as_f32_3(&self) -> [f32; 3] {
        match self.try_as_f32_3() {
            Ok(elements) => elements,
            Err(error) => panic!("{}", error),
        }
    }

    /// Convert the value to an array of three f32s.
    pub fn try_as_f32_3(&self) -> Result<[f32; 3], ValueTypeError> {
        let elements = self.try_as_array_with_len(3)?;
        Ok([
            elements[0].try_as_f32()?,
            elements[1].try_as_f32()?,
            elements[2].try_as_f32()?,
        ])
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            write!(f, "null")
        } else {
            write!(f, "@{:#X}", self.0)
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::None => write!(f, "none"),
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(r) => write!(f, "{}", r),
            Value::String(s) => write!(f, "{:?}", s),
            Value::Address(a) => write!(f, "{}", a),
            Value::Struct(fields) => {
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

impl From<()> for Value {
    fn from((): ()) -> Self {
        Self::None
    }
}

macro_rules! value_from_int {
    ($ty:ty) => {
        impl From<$ty> for Value {
            fn from(v: $ty) -> Self {
                Self::Int(v.into())
            }
        }
    };
}

value_from_int!(i8);
value_from_int!(u8);
value_from_int!(i16);
value_from_int!(u16);
value_from_int!(i32);
value_from_int!(u32);
value_from_int!(i64);
value_from_int!(u64);
value_from_int!(i128);

impl From<usize> for Value {
    fn from(v: usize) -> Self {
        Self::Int(v as IntValue)
    }
}

impl From<isize> for Value {
    fn from(v: isize) -> Self {
        Self::Int(v as IntValue)
    }
}

macro_rules! value_from_float {
    ($ty:ty) => {
        impl From<$ty> for Value {
            fn from(v: $ty) -> Self {
                Self::Float(v.into())
            }
        }
    };
}

value_from_float!(f32);
value_from_float!(f64);

impl From<String> for Value {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Self::String(v.to_string())
    }
}

impl From<Address> for Value {
    fn from(v: Address) -> Self {
        Self::Address(v)
    }
}

impl<V: Into<Value>> From<HashMap<String, V>> for Value {
    fn from(v: HashMap<String, V>) -> Self {
        Self::Struct(Box::new(
            v.into_iter().map(|(k, v)| (k, v.into())).collect(),
        ))
    }
}

impl<V: Into<Value>> From<HashMap<&str, V>> for Value {
    fn from(v: HashMap<&str, V>) -> Self {
        Self::Struct(Box::new(
            v.into_iter()
                .map(|(k, v)| (k.to_string(), v.into()))
                .collect(),
        ))
    }
}

impl<T: Into<Value>, const N: usize> From<[(String, T); N]> for Value {
    fn from(v: [(String, T); N]) -> Self {
        IntoIter::new(v).collect::<HashMap<_, _>>().into()
    }
}

impl<T: Into<Value> + Clone> From<&[(String, T)]> for Value {
    fn from(v: &[(String, T)]) -> Self {
        v.iter().cloned().collect::<HashMap<_, _>>().into()
    }
}

impl<T: Into<Value>> From<Vec<(String, T)>> for Value {
    fn from(v: Vec<(String, T)>) -> Self {
        v.into_iter().collect::<HashMap<_, _>>().into()
    }
}

impl<T: Into<Value>, const N: usize> From<[(&str, T); N]> for Value {
    fn from(v: [(&str, T); N]) -> Self {
        IntoIter::new(v).collect::<HashMap<_, _>>().into()
    }
}

impl<T: Into<Value> + Clone> From<&[(&str, T)]> for Value {
    fn from(v: &[(&str, T)]) -> Self {
        v.iter().cloned().collect::<HashMap<_, _>>().into()
    }
}

impl<T: Into<Value>> From<Vec<(&str, T)>> for Value {
    fn from(v: Vec<(&str, T)>) -> Self {
        v.into_iter().collect::<HashMap<_, _>>().into()
    }
}

impl<T: Into<Value>, const N: usize> From<[T; N]> for Value {
    fn from(v: [T; N]) -> Self {
        Self::Array(IntoIter::new(v).map(|v| v.into()).collect())
    }
}

impl<T: Into<Value> + Clone> From<&[T]> for Value {
    fn from(v: &[T]) -> Self {
        Self::Array(v.iter().map(|v| v.clone().into()).collect())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Self::Array(v.into_iter().map(|v| v.into()).collect())
    }
}
