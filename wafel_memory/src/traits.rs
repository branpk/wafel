use std::{collections::HashMap, ops::Deref};

use wafel_data_type::{
    Address, DataType, DataTypeRef, FloatType, FloatValue, IntType, IntValue, TypeName, Value,
};

use crate::MemoryError::{self, *};

/// Trait for looking up a symbol's address.
///
/// A symbol is the name of a global variable or function.
pub trait SymbolLookup {
    /// Look up a symbol's address.
    ///
    /// Returns None if the symbol is undefined.
    fn symbol_address(&self, symbol: &str) -> Option<Address>;
}

impl<R, M> SymbolLookup for R
where
    R: Deref<Target = M>,
    M: SymbolLookup,
{
    fn symbol_address(&self, symbol: &str) -> Option<Address> {
        self.deref().symbol_address(symbol)
    }
}

/// Trait for a view of memory that allows reading values by address.
///
/// Endianness should be handled by the implementer.
pub trait MemoryRead {
    /// Read an int from the given address.
    ///
    /// The int's size and signedness is given by `int_type`.
    fn read_int(&self, address: Address, int_type: IntType) -> Result<IntValue, MemoryError>;

    /// Read a float from the given address.
    ///
    /// The float's size and signedness is given by `float_type`.
    fn read_float(
        &self,
        address: Address,
        float_type: FloatType,
    ) -> Result<FloatValue, MemoryError>;

    /// Read an address value from the given address.
    ///
    /// The resulting address may be invalid or zero.
    fn read_address(&self, address: Address) -> Result<Address, MemoryError>;

    /// Read a value of the given type.
    ///
    /// Any type names present in `data_type` are resolved using `resolve_type`.
    ///
    /// This method can handle all data types except for:
    /// - Unsized arrays
    /// - Unions
    fn read_value(
        &self,
        address: Address,
        data_type: &DataTypeRef,
        mut resolve_type: impl FnMut(&TypeName) -> Option<DataTypeRef>,
    ) -> Result<Value, MemoryError> {
        read_value_impl(self, address, data_type, &mut resolve_type)
    }

    /// Read a null terminated C string from the given address.
    fn read_string(&self, address: Address) -> Result<Vec<u8>, MemoryError> {
        let mut bytes = Vec::new();
        let mut current = address;
        loop {
            let byte = self.read_int(current, IntType::U8)? as u8;
            if byte == 0 {
                break;
            }
            bytes.push(byte);
            current = current + 1;
        }
        Ok(bytes)
    }
}

fn read_value_impl<M: MemoryRead + ?Sized>(
    memory: &M,
    address: Address,
    data_type: &DataTypeRef,
    resolve_type: &mut impl FnMut(&TypeName) -> Option<DataTypeRef>,
) -> Result<Value, MemoryError> {
    let value = match data_type.as_ref() {
        DataType::Void => Value::None,
        DataType::Int(int_type) => Value::Int(memory.read_int(address, *int_type)?),
        DataType::Float(float_type) => Value::Float(memory.read_float(address, *float_type)?),
        DataType::Pointer { .. } => Value::Address(memory.read_address(address)?),
        DataType::Array {
            base,
            length,
            stride,
        } => match *length {
            Some(length) => {
                let values: Vec<Value> = (0..length)
                    .map(|index| {
                        read_value_impl(memory, address + index * *stride, base, resolve_type)
                    })
                    .collect::<Result<_, MemoryError>>()?;
                Value::Array(values)
            }
            None => return Err(ReadUnsizedArray),
        },
        DataType::Struct { fields } => {
            let mut field_values: HashMap<String, Value> = HashMap::new();
            for (name, field) in fields {
                let field_value = read_value_impl(
                    memory,
                    address + field.offset,
                    &field.data_type,
                    resolve_type,
                )?;
                field_values.insert(name.clone(), field_value);
            }
            Value::Struct(Box::new(field_values))
        }
        DataType::Union { .. } => return Err(ReadUnion),
        DataType::Name(type_name) => {
            let resolved_type =
                resolve_type(type_name).ok_or_else(|| UndefinedTypeName(type_name.clone()))?;
            read_value_impl(memory, address, &resolved_type, resolve_type)?
        }
    };
    Ok(value)
}

/// Trait for a view of memory that allows writing values by address.
///
/// Endianness should be handled by the implementer.
pub trait MemoryWrite {
    /// Write an int at the given address.
    ///
    /// The int's size and signedness is given by `int_type`.
    fn write_int(
        &mut self,
        address: Address,
        int_type: IntType,
        value: IntValue,
    ) -> Result<(), MemoryError>;

    /// Write a float at the given address.
    ///
    /// The float's size and signedness is given by `float_type`.
    fn write_float(
        &mut self,
        address: Address,
        float_type: FloatType,
        value: FloatValue,
    ) -> Result<(), MemoryError>;

    /// Write an address value at the given address.
    ///
    /// The address value may be invalid or zero.
    fn write_address(&mut self, address: Address, value: Address) -> Result<(), MemoryError>;

    /// Write a value of the given type.
    ///
    /// Any type names present in `data_type` are resolved using `resolve_type`.
    ///
    /// This method can handle all data types except for unions.
    fn write_value(
        &mut self,
        address: Address,
        data_type: &DataTypeRef,
        value: Value,
        mut resolve_type: impl FnMut(&TypeName) -> Option<DataTypeRef>,
    ) -> Result<(), MemoryError> {
        write_value_impl(self, address, data_type, value, &mut resolve_type)
    }
}

fn write_value_impl<M: MemoryWrite + ?Sized>(
    memory: &mut M,
    address: Address,
    data_type: &std::sync::Arc<DataType>,
    value: Value,
    resolve_type: &mut impl FnMut(&TypeName) -> Option<std::sync::Arc<DataType>>,
) -> Result<(), MemoryError> {
    match data_type.as_ref() {
        DataType::Void => value.try_as_none()?,
        DataType::Int(int_type) => {
            memory.write_int(address, *int_type, value.try_as_int_lenient()?)?
        }
        DataType::Float(float_type) => {
            memory.write_float(address, *float_type, value.try_as_float_lenient()?)?
        }
        DataType::Pointer { .. } => memory.write_address(address, value.try_as_address()?)?,
        DataType::Array {
            base,
            length,
            stride,
        } => {
            let elements = match *length {
                Some(length) => value.try_as_array_with_len(length)?,
                None => value.try_as_array()?,
            };
            for (i, element) in elements.iter().enumerate() {
                write_value_impl(
                    memory,
                    address + i * *stride,
                    base,
                    element.clone(),
                    resolve_type,
                )?;
            }
        }
        DataType::Struct { fields } => {
            let field_values = value.try_as_struct()?;
            for name in field_values.keys() {
                if !fields.contains_key(name) {
                    return Err(WriteExtraField(name.clone()));
                }
            }
            for name in fields.keys() {
                if !field_values.contains_key(name) {
                    return Err(WriteExtraField(name.clone()));
                }
            }
            for (field_name, field) in fields {
                let field_value = field_values[field_name].clone();
                write_value_impl(
                    memory,
                    address + field.offset,
                    &field.data_type,
                    field_value,
                    resolve_type,
                )?;
            }
        }
        DataType::Union { fields: _ } => return Err(WriteUnion),
        DataType::Name(type_name) => {
            let resolved_type =
                resolve_type(type_name).ok_or_else(|| UndefinedTypeName(type_name.clone()))?;
            write_value_impl(memory, address, &resolved_type, value, resolve_type)?
        }
    }
    Ok(())
}

/// A helper trait for implementing [MemoryRead].
pub trait MemoryReadPrimitive {
    /// Read a primitive value from memory.
    ///
    /// # Safety
    ///
    /// T must be an integral or float type.
    unsafe fn read_primitive<T: Copy>(&self, address: Address) -> Result<T, MemoryError>;

    /// Read an address from memory.
    fn read_address(&self, address: Address) -> Result<Address, MemoryError>;
}

/// A helper trait for implementing [MemoryWrite].
pub trait MemoryWritePrimitive {
    /// Write a primitive value to memory.
    ///
    /// # Safety
    ///
    /// T must be an integral or float type.
    unsafe fn write_primitive<T: Copy>(
        &mut self,
        address: Address,
        value: T,
    ) -> Result<(), MemoryError>;

    /// Write an address to memory.
    fn write_address(&mut self, address: Address, value: Address) -> Result<(), MemoryError>;
}

impl<M: MemoryReadPrimitive> MemoryRead for M {
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
        MemoryReadPrimitive::read_address(self, address)
    }
}

impl<M: MemoryWritePrimitive> MemoryWrite for M {
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
        MemoryWritePrimitive::write_address(self, address, value)
    }
}

/// A trait that allows accessing game memory and saving/loading states.
///
/// The memory is divided into static and non-static memory.
/// Static memory is considered immutable, e.g. the .code and .rodata sections, while
/// non-static memory is mutable and includes the .data and .bss sections.
///
/// A "slot" is a buffer that can hold a copy of non-static memory.
/// Each [GameMemory] comes with a base slot, which represents the game's actual loaded memory.
/// Other backup slots can be created, and data can be copied to/from the base slot.
pub trait GameMemory {
    /// A buffer that can hold a copy of non-static memory.
    type Slot;

    /// A read-only view of shared static memory.
    ///
    /// Attempting to read a non-static address using this will fail.
    type StaticView<'a>: MemoryRead
    where
        Self: 'a;

    /// A read-only view of both static and non-static memory, backed by a
    /// particular slot.
    ///
    /// Static addresses are looked up in shared static memory, while non-static
    /// addresses are looked up in the slot's buffer.
    type SlotView<'a>: MemoryRead
    where
        Self: 'a;

    /// A read-write view of both static and non-static memory, backed by a
    /// particular slot.
    ///
    /// Static addresses are looked up in shared static memory, while non-static
    /// addresses are looked up in the slot's buffer.
    type SlotViewMut<'a>: MemoryRead + MemoryWrite
    where
        Self: 'a;

    /// Return a read-only view of shared static memory.
    ///
    /// Attempting to read a non-static address using this will fail.
    fn static_view(&self) -> Self::StaticView<'_>;

    /// Return a read-only view of both static and non-static memory, backed by the given
    /// slot.
    ///
    /// Static addresses are looked up in shared static memory, while non-static
    /// addresses are looked up in the slot's buffer.
    fn with_slot<'a>(&'a self, slot: &'a Self::Slot) -> Self::SlotView<'a>;

    /// Return a read-write view of both static and non-static memory, backed by the given
    /// slot.
    ///
    /// Static addresses are looked up in shared static memory, while non-static
    /// addresses are looked up in the slot's buffer.
    fn with_slot_mut<'a>(&'a self, slot: &'a mut Self::Slot) -> Self::SlotViewMut<'a>;

    /// Allocate a new backup slot.
    ///
    /// Note that a slot can be large, so care should be taken to avoid allocating
    /// too many. An SM64 slot is ~2 MB.
    fn create_backup_slot(&self) -> Self::Slot;

    /// Copy data from one slot to another.
    ///
    /// # Panics
    ///
    /// Panics if either slot is not owned by this [GameMemory] instance.
    fn copy_slot(&self, dst: &mut Self::Slot, src: &Self::Slot);

    /// Advance the base slot by one frame.
    ///
    /// # Panics
    ///
    /// Panics if the slot is not the base slot, i.e. was created using
    /// [create_backup_slot](GameMemory::create_backup_slot).
    /// To advance a backup slot, you should first copy it to the base slot,
    /// then advance the base slot, then copy it back.
    fn advance_base_slot(&self, base_slot: &mut Self::Slot);
}

impl<R, M> GameMemory for R
where
    R: Deref<Target = M>,
    M: GameMemory + 'static,
{
    type Slot = M::Slot;

    type StaticView<'a>
    where
        R: 'a,
    = M::StaticView<'a>;

    type SlotView<'a>
    where
        R: 'a,
    = M::SlotView<'a>;

    type SlotViewMut<'a>
    where
        R: 'a,
    = M::SlotViewMut<'a>;

    fn static_view(&self) -> Self::StaticView<'_> {
        self.deref().static_view()
    }

    fn with_slot<'a>(&'a self, slot: &'a Self::Slot) -> Self::SlotView<'a> {
        self.deref().with_slot(slot)
    }

    fn with_slot_mut<'a>(&'a self, slot: &'a mut Self::Slot) -> Self::SlotViewMut<'a> {
        self.deref().with_slot_mut(slot)
    }

    fn create_backup_slot(&self) -> Self::Slot {
        self.deref().create_backup_slot()
    }

    fn copy_slot(&self, dst: &mut Self::Slot, src: &Self::Slot) {
        self.deref().copy_slot(dst, src)
    }

    fn advance_base_slot(&self, base_slot: &mut Self::Slot) {
        self.deref().advance_base_slot(base_slot)
    }
}
