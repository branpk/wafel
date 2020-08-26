//! Interface for interacting with program memory.

use super::{
    data_type::{DataType, DataTypeRef, FloatType, IntType},
    DataLayout, FloatValue, IntValue, MemoryErrorCause, Value,
};
use crate::{
    data_path::{DataPath, DataPathCache, GlobalDataPath, LocalDataPath},
    error::Error,
};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug, ops::Add};

/// A trait that defines the interface for interacting with a target program's memory.
///
/// Conceptually, memory is broken up into multiple parallel "slots". In practice,
/// each slot represents a physical buffer that contains a copy of the target program's
/// memory. There may also be static memory that doesn't reside in any slot.
///
/// The memory has one or more "base slots" that are capable of being frame advanced,
/// but can create backup slots to hold copies of the base slot's data.
pub trait Memory: Sized {
    /// The type of a slot.
    type Slot: Debug;

    /// The type of a static address that lies outside of slot memory.
    type StaticAddress;

    /// The type of an address that can be relocated to any slot.
    type RelocatableAddress;

    /// Read an integer from slot memory.
    ///
    /// The required size can be determined from `int_type`.
    fn read_slot_int(
        &self,
        slot: &Self::Slot,
        address: &Self::RelocatableAddress,
        int_type: IntType,
    ) -> Result<IntValue, Error>;

    /// Read a float from slot memory.
    ///
    /// The required size can be determined from `float_type`.
    fn read_slot_float(
        &self,
        slot: &Self::Slot,
        address: &Self::RelocatableAddress,
        float_type: FloatType,
    ) -> Result<FloatValue, Error>;

    /// Read an address from slot memory.
    fn read_slot_address(
        &self,
        slot: &Self::Slot,
        address: &Self::RelocatableAddress,
    ) -> Result<Address, Error>;

    /// Read an integer from static memory.
    ///
    /// The required size can be determined from `int_type`.
    fn read_static_int(
        &self,
        address: &Self::StaticAddress,
        int_type: IntType,
    ) -> Result<IntValue, Error>;

    /// Read a float from static memory.
    ///
    /// The required size can be determined from `float_type`.
    fn read_static_float(
        &self,
        address: &Self::StaticAddress,
        float_type: FloatType,
    ) -> Result<FloatValue, Error>;

    /// Read an address from static memory.
    fn read_static_address(&self, address: &Self::StaticAddress) -> Result<Address, Error>;

    /// Read an int from either static or slot memory.
    fn read_int(
        &self,
        slot: &Self::Slot,
        address: &ClassifiedAddress<Self>,
        int_type: IntType,
    ) -> Result<IntValue, Error> {
        match address {
            ClassifiedAddress::Static(address) => self.read_static_int(address, int_type),
            ClassifiedAddress::Relocatable(address) => self.read_slot_int(slot, address, int_type),
            ClassifiedAddress::Invalid => Err(MemoryErrorCause::InvalidAddress.into()),
        }
    }

    /// Read a float from either static or slot memory.
    fn read_float(
        &self,
        slot: &Self::Slot,
        address: &ClassifiedAddress<Self>,
        float_type: FloatType,
    ) -> Result<FloatValue, Error> {
        match address {
            ClassifiedAddress::Static(address) => self.read_static_float(address, float_type),
            ClassifiedAddress::Relocatable(address) => {
                self.read_slot_float(slot, address, float_type)
            }
            ClassifiedAddress::Invalid => Err(MemoryErrorCause::InvalidAddress.into()),
        }
    }

    /// Read an address from either static or slot memory.
    fn read_address(
        &self,
        slot: &Self::Slot,
        address: &ClassifiedAddress<Self>,
    ) -> Result<Address, Error> {
        match address {
            ClassifiedAddress::Static(address) => self.read_static_address(address),
            ClassifiedAddress::Relocatable(address) => self.read_slot_address(slot, address),
            ClassifiedAddress::Invalid => Err(MemoryErrorCause::InvalidAddress.into()),
        }
    }

    /// Write an int to slot memory.
    ///
    /// The size can be determined from `int_type`.
    /// Note that `value` may lie outside the range of `int_type`. Any necessary truncation should
    /// be done within this method.
    fn write_slot_int(
        &self,
        slot: &mut Self::Slot,
        address: &Self::RelocatableAddress,
        int_type: IntType,
        value: IntValue,
    ) -> Result<(), Error>;

    /// Write a float to slot memory.
    ///
    /// The size can be determined from `float_type`.
    /// Note that `value` may lie outside the range of `float_type`. Any necessary truncation
    /// should be done within this method.
    fn write_slot_float(
        &self,
        slot: &mut Self::Slot,
        address: &Self::RelocatableAddress,
        float_type: FloatType,
        value: FloatValue,
    ) -> Result<(), Error>;

    /// Write an address to memory.
    fn write_slot_address(
        &self,
        slot: &mut Self::Slot,
        address: &Self::RelocatableAddress,
        value: &Address,
    ) -> Result<(), Error>;

    /// Determine whether an address is static or can be relocated to a slot.
    ///
    /// This method should return ClassifiedAddress::Invalid for a null or invalid
    /// address, rather than returning an error.
    fn classify_address(&self, address: &Address) -> ClassifiedAddress<Self>;

    /// Read a value of type `data_type` from either slot or static memory.
    ///
    /// The default implementation only handles a subset of data types, and returns
    /// `MemoryError::UnreadableValue` for the rest.
    ///
    /// If `address` is null or invalid (according to `classify_address`),
    /// `MemoryError::InvalidAddress` is returned.
    fn read_value(
        &self,
        slot: &Self::Slot,
        address: &Address,
        data_type: &DataTypeRef,
    ) -> Result<Value, Error> {
        let data_type = self.data_layout().concrete_type(data_type)?;

        Ok(match data_type.as_ref() {
            DataType::Int(int_type) => {
                let address = self.classify_address(address);
                Value::Int(self.read_int(slot, &address, *int_type)?)
            }
            DataType::Float(float_type) => {
                let address = self.classify_address(address);
                Value::Float(self.read_float(slot, &address, *float_type)?)
            }
            DataType::Pointer { .. } => {
                let address = self.classify_address(address);
                Value::Address(self.read_address(slot, &address)?.into())
            }
            DataType::Struct { fields } => {
                let field_values: HashMap<String, Value> = fields
                    .iter()
                    .map(|(name, field)| {
                        self.read_value(slot, &(address.clone() + field.offset), &field.data_type)
                            .map(|value| (name.clone(), value))
                    })
                    .collect::<Result<_, Error>>()?;
                Value::Struct {
                    fields: Box::new(field_values),
                }
            }
            DataType::Array {
                base,
                length: Some(length),
                stride,
            } => {
                let values: Vec<Value> = (0..*length)
                    .map(|index| self.read_value(slot, &(address.clone() + index * *stride), base))
                    .collect::<Result<_, Error>>()?;
                Value::Array(values)
            }
            _ => Err(MemoryErrorCause::UnreadableValue {
                data_type: data_type.clone(),
            })?,
        })
    }

    /// Write a value of type `data_type` to slot memory.
    ///
    /// The default implementation only handles a subset of data types, and returns
    /// MemoryError::UnwritableValue for the rest.
    ///
    /// It is not currently allowed to write to static memory since it can lead
    /// to unexpected results.
    ///
    /// If `address` is null or invalid (according to `classify_address`),
    /// `MemoryError::InvalidAddress` is returned.
    fn write_value(
        &self,
        slot: &mut Self::Slot,
        address: &Address,
        data_type: &DataTypeRef,
        value: &Value,
    ) -> Result<(), Error> {
        let to_relocatable = |address| -> Result<_, Error> {
            match self.classify_address(address) {
                ClassifiedAddress::Static(_) => Err(MemoryErrorCause::WriteToStaticAddress.into()),
                ClassifiedAddress::Relocatable(address) => Ok(address),
                ClassifiedAddress::Invalid => Err(MemoryErrorCause::InvalidAddress.into()),
            }
        };

        Ok(match data_type.as_ref() {
            DataType::Int(int_type) => {
                let address = to_relocatable(address)?;
                self.write_slot_int(slot, &address, *int_type, value.as_int()?)?;
            }
            DataType::Float(float_type) => {
                let address = to_relocatable(address)?;
                self.write_slot_float(slot, &address, *float_type, value.as_float()?)?;
            }
            DataType::Pointer { .. } => {
                let address = to_relocatable(address)?;
                self.write_slot_address(slot, &address, &value.as_address()?.into())?;
            }
            _ => Err(MemoryErrorCause::UnwritableValue {
                data_type: data_type.clone(),
            })?,
        })
    }

    /// Get the data type and global variable information for memory access.
    fn data_layout(&self) -> &DataLayout;

    /// Get a mutable reference to the data layout.
    fn data_layout_mut(&mut self) -> &mut DataLayout;

    /// Look up the address for a global variable by name.
    fn symbol_address(&self, symbol: &str) -> Result<Address, Error>;

    /// Return a data path cache.
    fn data_path_cache(&self) -> &DataPathCache;

    /// Look up or compile a data path (either global or local).
    fn data_path(&self, source: &str) -> Result<DataPath, Error> {
        self.data_path_cache().path(self, source)
    }

    /// Look up or compile a global data path using the cache.
    fn global_path(&self, source: &str) -> Result<GlobalDataPath, Error> {
        Ok(self.data_path(source)?.into_global()?)
    }

    /// Look up or compile a local data path using the cache.
    fn local_path(&self, source: &str) -> Result<LocalDataPath, Error> {
        Ok(self.data_path(source)?.into_local()?)
    }

    /// Allocate a new backup slot.
    fn create_backup_slot(&self) -> Result<Self::Slot, Error>;

    /// Copy the contents of one slot into another.
    fn copy_slot(&self, dst: &mut Self::Slot, src: &Self::Slot) -> Result<(), Error>;

    /// Advance a base slot one frame.
    fn advance_base_slot(&self, base_slot: &mut Self::Slot) -> Result<(), Error>;
}

/// A raw pointer value that can be stored in memory.
///
/// Having a single numeric type is convenient so that `Value` doesn't have to be generic
/// on a `Memory` implementation.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[display(fmt = "{:#X}", _0)]
pub struct Address(pub usize);

impl Add<usize> for Address {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0.wrapping_add(rhs))
    }
}

/// An address that has been classified as either static or relocatable.
#[derive(Debug)]
pub enum ClassifiedAddress<M: Memory> {
    /// A static address that lies outside of any slot.
    Static(M::StaticAddress),
    /// An address that can be relocated to a specific slot.
    Relocatable(M::RelocatableAddress),
    /// A null or invalid address.
    Invalid,
}
