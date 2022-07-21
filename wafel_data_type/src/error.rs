#![allow(missing_docs)]

use std::{borrow::Cow, error::Error, fmt};

use crate::{DataType, Value};

#[derive(Debug, Clone)]
pub enum DataTypeError {
    ExpectedType {
        expected: Cow<'static, str>,
        actual: DataType,
    },
    NoSuchField {
        name: String,
    },
}

impl fmt::Display for DataTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataTypeError::ExpectedType { expected, actual } => {
                write!(f, "expected {}, found {}", expected, actual)
            }
            DataTypeError::NoSuchField { name } => {
                write!(f, "no such field: {}", name)
            }
        }
    }
}

impl Error for DataTypeError {}

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
