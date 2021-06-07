#![allow(incomplete_features)]
#![feature(generic_associated_types)]

use std::{collections::HashMap, error::Error, fmt};

use wafel_data_type::{
    Address, DataType, DataTypeRef, FloatType, FloatValue, IntType, IntValue, Value,
};
use wafel_layout::{DataLayout, LayoutLookupError};

pub mod data_path;
pub mod dll;

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
                return Err(MemoryError::UnreadableValue(data_type.clone()));
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
    fn write_address(&mut self, address: Address, value: Address) -> Result<(), MemoryError>;

    fn write_value(
        &mut self,
        address: Address,
        data_type: &DataTypeRef,
        value: Value,
    ) -> Result<(), MemoryError> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub enum MemoryError {
    Context {
        context: String,
        error: Box<MemoryError>,
    },
    LayoutLookupError(LayoutLookupError),
    UnreadableValue(DataTypeRef),
    UnwritableValue(DataTypeRef),
    InvalidAddress,
    WriteToStaticAddress,
    NonStaticAddressInStaticView,
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::Context { context, error } => write!(f, "{}: {}", context, error),
            MemoryError::LayoutLookupError(error) => write!(f, "{}", error),
            MemoryError::UnreadableValue(data_type) => {
                write!(f, "cannot read value of type {}", data_type)
            }
            MemoryError::UnwritableValue(data_type) => {
                write!(f, "cannot write value of type {}", data_type)
            }
            MemoryError::InvalidAddress => write!(f, "null or invalid address"),
            MemoryError::WriteToStaticAddress => write!(f, "write to static address"),
            MemoryError::NonStaticAddressInStaticView => {
                write!(f, "using a non-static address through a static memory view")
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

pub trait SlottedMemory {
    type Slot;

    type StaticView<'a>: MemoryRead;
    type SlotView<'a>: MemoryRead;
    type SlotViewMut<'a>: MemoryRead + MemoryWrite;

    fn static_view(&self) -> Self::StaticView<'_>;
    fn with_slot<'a>(&'a self, slot: &'a Self::Slot) -> Self::SlotView<'a>;
    fn with_slot_mut<'a>(&'a self, slot: &'a mut Self::Slot) -> Self::SlotViewMut<'a>;

    fn create_backup_slot(&self) -> Self::Slot;
    fn copy_slot(&self, dst: &mut Self::Slot, src: &Self::Slot);
    fn advance_base_slot(&self, base_slot: &mut Self::Slot);
}

pub trait MemoryPrimitiveRead {
    /// Read a primitive value from memory.
    ///
    /// # Safety
    ///
    /// T must be an integral, float, or raw pointer type.
    unsafe fn read_primitive<T: Copy>(&self, address: Address) -> Result<T, MemoryError>;
}

pub trait MemoryPrimitiveWrite {
    /// Write a primitive value to memory.
    ///
    /// # Safety
    ///
    /// T must be an integral, float, or raw pointer type.
    unsafe fn write_primitive<T: Copy>(
        &mut self,
        address: Address,
        value: T,
    ) -> Result<(), MemoryError>;
}

impl<M: MemoryPrimitiveRead> MemoryRead for M {
    fn read_int(&self, address: Address, int_type: IntType) -> Result<IntValue, MemoryError> {
        unsafe {
            Ok(match int_type {
                IntType::U8 => (self.read_primitive::<u8>(address)?).into(),
                IntType::S8 => (self.read_primitive::<i8>(address)?).into(),
                IntType::U16 => (self.read_primitive::<u16>(address)?).into(),
                IntType::S16 => (self.read_primitive::<i16>(address)?).into(),
                IntType::U32 => (self.read_primitive::<u32>(address)?).into(),
                IntType::S32 => (self.read_primitive::<i32>(address)?).into(),
                IntType::U64 => (self.read_primitive::<u64>(address)?).into(),
                IntType::S64 => (self.read_primitive::<i64>(address)?).into(),
            })
        }
    }

    fn read_float(
        &self,
        address: Address,
        float_type: FloatType,
    ) -> Result<FloatValue, MemoryError> {
        unsafe {
            Ok(match float_type {
                FloatType::F32 => (self.read_primitive::<f32>(address)?).into(),
                FloatType::F64 => (self.read_primitive::<f64>(address)?),
            })
        }
    }

    fn read_address(&self, address: Address) -> Result<Address, MemoryError> {
        unsafe {
            let pointer = self.read_primitive::<*const u8>(address)?;
            Ok(Address(pointer as usize))
        }
    }
}

impl<M: MemoryPrimitiveWrite> MemoryWrite for M {
    fn write_int(
        &mut self,
        address: Address,
        int_type: IntType,
        value: IntValue,
    ) -> Result<(), MemoryError> {
        unsafe {
            match int_type {
                IntType::U8 => self.write_primitive(address, value as u8),
                IntType::S8 => self.write_primitive(address, value as i8),
                IntType::U16 => self.write_primitive(address, value as u16),
                IntType::S16 => self.write_primitive(address, value as i16),
                IntType::U32 => self.write_primitive(address, value as u32),
                IntType::S32 => self.write_primitive(address, value as i32),
                IntType::U64 => self.write_primitive(address, value as u64),
                IntType::S64 => self.write_primitive(address, value as i64),
            }
        }
    }

    fn write_float(
        &mut self,
        address: Address,
        float_type: FloatType,
        value: FloatValue,
    ) -> Result<(), MemoryError> {
        unsafe {
            match float_type {
                FloatType::F32 => self.write_primitive(address, value as f32),
                FloatType::F64 => self.write_primitive(address, value as f64),
            }
        }
    }

    fn write_address(&mut self, address: Address, value: Address) -> Result<(), MemoryError> {
        unsafe { self.write_primitive(address, value.0 as *const u8) }
    }
}
