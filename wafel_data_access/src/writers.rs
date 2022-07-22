#![allow(clippy::needless_range_loop)]
#![allow(missing_docs)]

use std::num::Wrapping;

use indexmap::IndexMap;
use wafel_data_type::{Address, DataType, DataTypeRef, FloatType, IntType, TypeName, Value};
use wafel_memory::{MemoryRead, MemoryWrite};

use crate::{
    DataError::{self, *},
    DataStride, DataWritable, DataWriter, MemoryLayout,
};

macro_rules! prim_writable {
    ($ty:ident, $writer:ident, $method:ident, $prim_ty:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
        pub struct $writer;

        impl $writer {
            pub fn write<M: MemoryRead + MemoryWrite>(
                &self,
                memory: &mut M,
                addr: Address,
                value: &$ty,
            ) -> Result<(), DataError> {
                memory.$method(addr, $prim_ty, (*value).into())?;
                Ok(())
            }
        }

        impl DataWriter for $writer {
            type Value = $ty;

            fn write<M: MemoryRead + MemoryWrite>(
                &self,
                memory: &mut M,
                addr: Address,
                value: &$ty,
            ) -> Result<(), DataError> {
                self.write(memory, addr, value)
            }
        }

        impl DataWritable for $ty {
            type Writer = $writer;

            fn writer(_layout: &impl MemoryLayout) -> Result<$writer, DataError> {
                Ok($writer)
            }
        }
    };
}

prim_writable!(u8, U8Writer, write_int, IntType::U8);
prim_writable!(i8, I8Writer, write_int, IntType::S8);
prim_writable!(u16, U16Writer, write_int, IntType::U16);
prim_writable!(i16, I16Writer, write_int, IntType::S16);
prim_writable!(u32, U32Writer, write_int, IntType::U32);
prim_writable!(i32, I32Writer, write_int, IntType::S32);
prim_writable!(u64, U64Writer, write_int, IntType::U64);
prim_writable!(i64, I64Writer, write_int, IntType::S64);

prim_writable!(f32, F32Writer, write_float, FloatType::F32);
prim_writable!(f64, F64Writer, write_float, FloatType::F64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AddressWriter;

impl AddressWriter {
    pub fn write<M: MemoryRead + MemoryWrite>(
        &self,
        memory: &mut M,
        addr: Address,
        value: Address,
    ) -> Result<(), DataError> {
        memory.write_address(addr, value)?;
        Ok(())
    }
}

impl DataWriter for AddressWriter {
    type Value = Address;

    fn write<M: MemoryRead + MemoryWrite>(
        &self,
        memory: &mut M,
        addr: Address,
        value: &Address,
    ) -> Result<(), DataError> {
        self.write(memory, addr, *value)
    }
}

impl DataWritable for Address {
    type Writer = AddressWriter;

    fn writer(_layout: &impl MemoryLayout) -> Result<AddressWriter, DataError> {
        Ok(AddressWriter)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ArrayWriter<W, const N: usize> {
    inner: W,
    stride: usize,
}

impl<W, const N: usize> ArrayWriter<W, N>
where
    W: DataWriter,
{
    pub fn write<M: MemoryRead + MemoryWrite>(
        &self,
        memory: &mut M,
        addr: Address,
        value: &[W::Value; N],
    ) -> Result<(), DataError> {
        for i in 0..N {
            self.inner
                .write(memory, addr + i * self.stride, &value[i])?;
        }
        Ok(())
    }
}

impl<W, const N: usize> DataWriter for ArrayWriter<W, N>
where
    W: DataWriter,
{
    type Value = [W::Value; N];

    fn write<M: MemoryRead + MemoryWrite>(
        &self,
        memory: &mut M,
        addr: Address,
        value: &[W::Value; N],
    ) -> Result<(), DataError> {
        self.write(memory, addr, value)
    }
}

impl<T, const N: usize> DataWritable for [T; N]
where
    T: DataWritable + DataStride,
{
    type Writer = ArrayWriter<T::Writer, N>;

    fn writer(layout: &impl MemoryLayout) -> Result<Self::Writer, DataError> {
        Ok(ArrayWriter {
            inner: T::writer(layout)?,
            stride: T::stride(layout)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DataTypeWriter {
    pub(crate) data_type: DataTypeRef,
    pub(crate) concrete_types: IndexMap<TypeName, DataTypeRef>,
}

impl DataTypeWriter {
    pub fn write<M: MemoryRead + MemoryWrite>(
        &self,
        memory: &mut M,
        addr: Address,
        value: &Value,
    ) -> Result<(), DataError> {
        write_value_impl(memory, addr, &self.data_type, value, &self.concrete_types)
    }
}

impl DataWriter for DataTypeWriter {
    type Value = Value;

    fn write<M: MemoryRead + MemoryWrite>(
        &self,
        memory: &mut M,
        addr: Address,
        value: &Value,
    ) -> Result<(), DataError> {
        self.write(memory, addr, value)
    }
}

pub(crate) fn write_value_impl(
    memory: &mut impl MemoryWrite,
    address: Address,
    data_type: &DataTypeRef,
    value: &Value,
    concrete_types: &IndexMap<TypeName, DataTypeRef>,
) -> Result<(), DataError> {
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
                write_value_impl(memory, address + i * *stride, base, element, concrete_types)?;
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
                write_value_impl(
                    memory,
                    address + field.offset,
                    &field.data_type,
                    &field_values[field_name],
                    concrete_types,
                )?;
            }
        }
        DataType::Union { fields: _ } => return Err(WriteUnion),
        DataType::Name(type_name) => {
            let resolved_type = concrete_types
                .get(type_name)
                .expect("missing concrete type for type name");
            write_value_impl(memory, address, resolved_type, value, concrete_types)?
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct WrappingWriter<W>(W);

impl<W> WrappingWriter<W>
where
    W: DataWriter,
{
    fn write<M: MemoryRead + MemoryWrite>(
        &self,
        memory: &mut M,
        addr: Address,
        value: &Wrapping<W::Value>,
    ) -> Result<(), DataError> {
        self.0.write(memory, addr, &value.0)
    }
}

impl<W> DataWriter for WrappingWriter<W>
where
    W: DataWriter,
{
    type Value = Wrapping<W::Value>;

    fn write<M: MemoryRead + MemoryWrite>(
        &self,
        memory: &mut M,
        addr: Address,
        value: &Wrapping<W::Value>,
    ) -> Result<(), DataError> {
        self.write(memory, addr, value)
    }
}

impl<T> DataWritable for Wrapping<T>
where
    T: DataWritable,
{
    type Writer = WrappingWriter<T::Writer>;

    fn writer(layout: &impl MemoryLayout) -> Result<Self::Writer, DataError> {
        T::writer(layout).map(WrappingWriter)
    }
}
