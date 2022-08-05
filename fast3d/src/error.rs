#![allow(missing_docs)]

use core::fmt;
use std::error;

#[derive(Debug, Clone)]
pub enum F3DError {
    InvalidCommand([u32; 2]),
    InvalidVertexIndex,
    InvalidTMemOffset,
    MissingSetTextureImage,
    MultipleFillColors,
}

impl fmt::Display for F3DError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            F3DError::InvalidCommand([w0, w1]) => {
                write!(f, "invalid F3D command: {:#010X} {:#010X}", w0, w1)
            }
            F3DError::InvalidVertexIndex => write!(f, "invalid vertex index"),
            F3DError::InvalidTMemOffset => write!(f, "invalid tmem offset"),
            F3DError::MissingSetTextureImage => {
                write!(f, "LoadBlock without SetTextureImage")
            }
            F3DError::MultipleFillColors => write!(f, "unimplemented: multiple fill colors"),
        }
    }
}

impl error::Error for F3DError {}
