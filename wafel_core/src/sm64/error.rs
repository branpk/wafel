//! Error definitions.

#![allow(missing_docs)]

use derive_more::{Display, Error, From};

#[derive(Debug, Display, Error, From)]
pub enum SM64ErrorCause {
    #[display(fmt = "unhandled variable: {}", variable)]
    UnhandledVariable { variable: String },
    #[display(fmt = "variable is missing frame: {}", variable)]
    MissingFrame { variable: String },
    #[display(fmt = "variable is missing object: {}", variable)]
    MissingObject { variable: String },
    #[display(fmt = "variable is missing surface: {}", variable)]
    MissingSurface { variable: String },
    #[display(fmt = "unsupported conversion from {} to data value", value)]
    ValueFromPython { value: String },
    #[from]
    VariableSerdeError(serde_json::Error),
}
