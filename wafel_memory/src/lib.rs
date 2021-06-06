#![allow(incomplete_features)]
#![feature(generic_associated_types)]

use std::{collections::HashMap, error::Error, fmt};

use wafel_data_type::{
    Address, DataType, DataTypeRef, FloatType, FloatValue, IntType, IntValue, Value,
};
use wafel_layout::{DataLayout, LayoutLookupError};

pub mod data_path;

pub trait SymbolLookup {
    fn symbol_address(&self, symbol: &str) -> Option<Address>;
}

pub trait MemoryRead {
    fn read_int(&self, address: Address, int_type: IntType) -> Result<IntValue, MemoryError>;
    fn read_float(
        &self,
        address: Address,
        float_type: FloatType,
    ) -> Result<FloatValue, MemoryError>;
    fn read_address(&self, address: Address) -> Result<Address, MemoryError>;

    fn read_value(
        &self,
        address: Address,
        data_type: &DataTypeRef,
        layout: &DataLayout,
    ) -> Result<Value, MemoryError> {
        Ok(match data_type.as_ref() {
            DataType::Int(int_type) => Value::Int(self.read_int(address, *int_type)?),
            DataType::Float(float_type) => Value::Float(self.read_float(address, *float_type)?),
            DataType::Pointer { .. } => Value::Address(self.read_address(address)?),
            DataType::Array {
                base,
                length: Some(length),
                stride,
            } => {
                let values: Vec<Value> = (0..*length)
                    .map(|index| self.read_value(address + index * *stride, base, layout))
                    .collect::<Result<_, MemoryError>>()?;
                Value::Array(values)
            }
            DataType::Struct { fields } => {
                let mut field_values: HashMap<String, Value> = HashMap::new();
                for (name, field) in fields {
                    let field_value =
                        self.read_value(address + field.offset, &field.data_type, layout)?;
                    field_values.insert(name.clone(), field_value);
                }
                Value::Struct {
                    fields: Box::new(field_values),
                }
            }
            DataType::Name(type_name) => {
                let resolved_type = layout.data_type(type_name)?;
                self.read_value(address, resolved_type, layout)?
            }
            _ => {
                return Err(MemoryError::UnreadableValue {
                    data_type: data_type.clone(),
                });
            }
        })
    }
}

pub trait MemoryWrite {
    fn write_int(
        &mut self,
        address: Address,
        int_type: IntType,
        value: IntValue,
    ) -> Result<(), MemoryError>;
    fn write_float(
        &mut self,
        address: Address,
        float_type: FloatType,
        value: FloatValue,
    ) -> Result<(), MemoryError>;
    fn write_address(&mut self, address: Address) -> Result<(), MemoryError>;

    fn write_value(
        &mut self,
        address: Address,
        data_type: &DataTypeRef,
        value: Value,
    ) -> Result<(), MemoryError>;
}

#[derive(Debug, Clone)]
pub enum MemoryError {
    Context {
        context: String,
        error: Box<MemoryError>,
    },
    LayoutLookupError(LayoutLookupError),
    UnreadableValue {
        data_type: DataTypeRef,
    },
    UnwritableValue {
        data_type: DataTypeRef,
    },
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::Context { context, error } => write!(f, "{}: {}", context, error),
            MemoryError::LayoutLookupError(error) => write!(f, "{}", error),
            MemoryError::UnreadableValue { data_type } => {
                write!(f, "cannot read value of type {}", data_type)
            }
            MemoryError::UnwritableValue { data_type } => {
                write!(f, "cannot write value of type {}", data_type)
            }
        }
    }
}

impl Error for MemoryError {}

impl From<LayoutLookupError> for MemoryError {
    fn from(v: LayoutLookupError) -> Self {
        Self::LayoutLookupError(v)
    }
}

// pub trait SlottedMemory {
//     type Slot;
//     type StaticAddress;
//     type RelocatableAddress;
//
//     type StaticView<'a>: MemoryRead;
//     type SlotView<'a>: MemoryRead;
//     type SlotViewMut<'a>: MemoryRead + MemoryWrite;
//
//     fn static_view(&self) -> Self::StaticView<'_>;
//     fn with_slot<'a>(&'a self, slot: &'a Self::Slot) -> Self::SlotView<'a>;
//     fn with_slot_mut<'a>(&'a self, slot: &'a mut Self::Slot) -> Self::SlotViewMut<'a>;
//
//     fn create_backup_slot(&self) -> Self::Slot;
//     fn copy_slot(&self, dst: &mut Self::Slot, src: &Self::Slot);
//     fn advance_base_slot(&self, base_slot: &mut Self::Slot);
// }
