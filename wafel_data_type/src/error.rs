#![allow(missing_docs)]

use std::{borrow::Cow, error::Error, fmt};

use crate::Value;

#[derive(Debug, Clone)]
pub struct NotAnArrayOrPointerError;

impl fmt::Display for NotAnArrayOrPointerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DataType::stride called a non-array/pointer type")
    }
}

impl Error for NotAnArrayOrPointerError {}

#[derive(Debug, Clone)]
pub struct ValueTypeError {
    pub expected: Cow<'static, str>,
    pub actual: Value,
}

impl fmt::Display for ValueTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "expected value of type {}, found {}",
            self.expected, self.actual
        )
    }
}

impl Error for ValueTypeError {}
