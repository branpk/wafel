//! Error definitions.

#![allow(missing_docs)]

use derive_more::{Display, Error};
use wafel_types::{DataTypeRef, TypeName};

#[derive(Debug, Display, Error)]
pub enum MemoryErrorCause {
    #[display(fmt = "undefined type name {}", name)]
    UndefinedTypeName { name: TypeName },
    #[display(fmt = "undefined global {}", name)]
    UndefinedGlobal { name: String },
    #[display(fmt = "undefined constant {}", name)]
    UndefinedConstant { name: String },
    #[display(fmt = "value {} has incorrect type; expected {}", value, expected)]
    ValueTypeError { value: String, expected: String },
    #[display(fmt = "cannot read value of type {}", data_type)]
    UnreadableValue { data_type: DataTypeRef },
    #[display(fmt = "cannot write value of type {}", data_type)]
    UnwritableValue { data_type: DataTypeRef },
    #[display(fmt = "cannot write to static memory")]
    WriteToStaticAddress,
    #[display(fmt = "base slot is required, used {}", slot)]
    NonBaseSlot { slot: String },
    #[display(fmt = "using slot allocated from wrong memory")]
    SlotFromDifferentMemory,
    #[display(fmt = "null or invalid address")]
    InvalidAddress,
}
