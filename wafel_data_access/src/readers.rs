#![allow(clippy::needless_range_loop)]
#![allow(missing_docs)]

use core::array;
use std::{mem, num::Wrapping};

use indexmap::IndexMap;
use wafel_data_type::{Address, DataType, DataTypeRef, TypeName, Value};
use wafel_memory::MemoryRead;

use crate::{
    DataError::{self, *},
    DataReadable, DataReader, DataStride, MemoryLayout,
};

// TODO: Arrays should determine stride based on the field type in derive?

macro_rules! prim_readable {
    ($ty:ident, $reader:ident, $method:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
        pub struct $reader;

        impl $reader {
            pub fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<$ty, DataError> {
                Ok(memory.$method(addr)?)
            }
        }

        impl DataReader for $reader {
            type Value = $ty;

            fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<$ty, DataError> {
                self.read(memory, addr)
            }
        }

        impl DataReadable for $ty {
            type Reader = $reader;

            fn reader(_layout: &impl MemoryLayout) -> Result<$reader, DataError> {
                Ok($reader)
            }
        }

        impl DataStride for $ty {
            fn stride(_layout: &impl MemoryLayout) -> Result<usize, DataError> {
                Ok(mem::size_of::<$ty>())
            }
        }
    };
}

prim_readable!(u8, U8Reader, read_u8);
prim_readable!(i8, I8Reader, read_i8);
prim_readable!(u16, U16Reader, read_u16);
prim_readable!(i16, I16Reader, read_i16);
prim_readable!(u32, U32Reader, read_u32);
prim_readable!(i32, I32Reader, read_i32);
prim_readable!(u64, U64Reader, read_u64);
prim_readable!(i64, I64Reader, read_i64);

prim_readable!(f32, F32Reader, read_f32);
prim_readable!(f64, F64Reader, read_f64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AddressReader;

impl AddressReader {
    pub fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<Address, DataError> {
        Ok(memory.read_addr(addr)? as Address)
    }
}

impl DataReader for AddressReader {
    type Value = Address;

    fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<Address, DataError> {
        self.read(memory, addr)
    }
}

impl DataReadable for Address {
    type Reader = AddressReader;

    fn reader(_layout: &impl MemoryLayout) -> Result<AddressReader, DataError> {
        Ok(AddressReader)
    }
}

impl DataStride for Address {
    fn stride(layout: &impl MemoryLayout) -> Result<usize, DataError> {
        Ok(layout.pointer_size())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ArrayReader<R, const N: usize> {
    inner: R,
    stride: usize,
}

impl<R, const N: usize> ArrayReader<R, N>
where
    R: DataReader,
    R::Value: Default,
{
    pub fn read(
        &self,
        memory: &impl MemoryRead,
        addr: Address,
    ) -> Result<[R::Value; N], DataError> {
        let mut result = array::from_fn(|_| Default::default());
        for i in 0..N {
            result[i] = self.inner.read(memory, addr + i * self.stride)?;
        }
        Ok(result)
    }
}

impl<R, const N: usize> DataReader for ArrayReader<R, N>
where
    R: DataReader,
    R::Value: Default,
{
    type Value = [R::Value; N];

    fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<[R::Value; N], DataError> {
        self.read(memory, addr)
    }
}

impl<T, const N: usize> DataReadable for [T; N]
where
    T: DataReadable + DataStride + Default,
{
    type Reader = ArrayReader<T::Reader, N>;

    fn reader(layout: &impl MemoryLayout) -> Result<Self::Reader, DataError> {
        Ok(ArrayReader {
            inner: T::reader(layout)?,
            stride: T::stride(layout)?,
        })
    }
}

impl<T, const N: usize> DataStride for [T; N]
where
    T: DataStride,
{
    fn stride(layout: &impl MemoryLayout) -> Result<usize, DataError> {
        Ok(T::stride(layout)? * N)
    }
}

#[derive(Debug, Clone)]
pub struct DataTypeReader {
    pub(crate) data_type: DataTypeRef,
    pub(crate) concrete_types: IndexMap<TypeName, DataTypeRef>,
}

impl DataTypeReader {
    pub fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<Value, DataError> {
        read_value_impl(memory, addr, &self.data_type, &self.concrete_types)
    }
}

impl DataReader for DataTypeReader {
    type Value = Value;

    fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<Value, DataError> {
        self.read(memory, addr)
    }
}

pub(crate) fn read_value_impl(
    memory: &impl MemoryRead,
    addr: Address,
    data_type: &DataTypeRef,
    concrete_types: &IndexMap<TypeName, DataTypeRef>,
) -> Result<Value, DataError> {
    let value = match data_type.as_ref() {
        DataType::Void => Value::None,
        DataType::Int(int_type) => Value::Int(memory.read_int(addr, *int_type)?),
        DataType::Float(float_type) => Value::Float(memory.read_float(addr, *float_type)?),
        DataType::Pointer { .. } => Value::Address(memory.read_addr(addr)?),
        DataType::Array {
            base,
            length,
            stride,
        } => match *length {
            Some(length) => {
                let values: Vec<Value> = (0..length)
                    .map(|index| {
                        read_value_impl(memory, addr + index * *stride, base, concrete_types)
                    })
                    .collect::<Result<_, DataError>>()?;
                Value::Array(values)
            }
            None => return Err(ReadUnsizedArray),
        },
        DataType::Struct { fields } => {
            let mut field_values: IndexMap<String, Value> = IndexMap::new();
            for (name, field) in fields {
                let field_value = read_value_impl(
                    memory,
                    addr + field.offset,
                    &field.data_type,
                    concrete_types,
                )?;
                field_values.insert(name.clone(), field_value);
            }
            Value::Struct(Box::new(field_values))
        }
        DataType::Union { .. } => return Err(ReadUnion),
        DataType::Name(type_name) => {
            let resolved_type = concrete_types
                .get(type_name)
                .expect("missing concrete type for type name");
            read_value_impl(memory, addr, resolved_type, concrete_types)?
        }
    };
    Ok(value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct WrappingReader<R>(R);

impl<R> WrappingReader<R>
where
    R: DataReader,
{
    pub fn read(
        &self,
        memory: &impl MemoryRead,
        addr: Address,
    ) -> Result<Wrapping<R::Value>, DataError> {
        self.0.read(memory, addr).map(Wrapping)
    }
}

impl<R> DataReader for WrappingReader<R>
where
    R: DataReader,
{
    type Value = Wrapping<R::Value>;

    fn read(
        &self,
        memory: &impl MemoryRead,
        addr: Address,
    ) -> Result<Wrapping<R::Value>, DataError> {
        self.read(memory, addr)
    }
}

impl<T> DataReadable for Wrapping<T>
where
    T: DataReadable,
{
    type Reader = WrappingReader<T::Reader>;

    fn reader(layout: &impl MemoryLayout) -> Result<Self::Reader, DataError> {
        T::reader(layout).map(WrappingReader)
    }
}

impl<T> DataStride for Wrapping<T>
where
    T: DataStride,
{
    fn stride(layout: &impl MemoryLayout) -> Result<usize, DataError> {
        T::stride(layout)
    }
}
