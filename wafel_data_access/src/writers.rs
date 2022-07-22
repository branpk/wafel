#![allow(clippy::needless_range_loop)]
#![allow(missing_docs)]

use indexmap::IndexMap;
use wafel_data_type::{Address, DataType, DataTypeRef, FloatType, IntType, TypeName, Value};
use wafel_memory::{MemoryRead, MemoryWrite};

use crate::{
    DataError::{self, *},
    DataWritable, DataWriter, MemoryLayout,
};

macro_rules! prim_writable {
    ($ty:ident, $writer:ident, $method:ident $(, $arg:expr)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
        pub struct $writer;

        impl $writer {
            pub fn write<M: MemoryRead + MemoryWrite>(
                &self,
                memory: &mut M,
                addr: Address,
                value: $ty,
            ) -> Result<(), DataError> {
                memory.$method(addr, $($arg,)* value.into())?;
                Ok(())
            }
        }

        impl DataWriter for $writer {
            type Value = $ty;

            fn write<M: MemoryRead + MemoryWrite>(
                &self,
                memory: &mut M,
                addr: Address,
                value: $ty,
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

prim_writable!(Address, AddressWriter, write_address);

macro_rules! prim_array_writable {
    ($ty:ident, $writer:ident, $array_writer:ident, $size:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
        pub struct $array_writer<const N: usize>;

        impl<const N: usize> $array_writer<N> {
            pub fn write<M: MemoryRead + MemoryWrite>(
                &self,
                memory: &mut M,
                addr: Address,
                value: [$ty; N],
            ) -> Result<(), DataError> {
                let stride = $size;
                for i in 0..N {
                    $writer.write(memory, addr + i * stride, value[i])?;
                }
                Ok(())
            }
        }

        impl<const N: usize> DataWriter for $array_writer<N> {
            type Value = [$ty; N];

            fn write<M: MemoryRead + MemoryWrite>(
                &self,
                memory: &mut M,
                addr: Address,
                value: [$ty; N],
            ) -> Result<(), DataError> {
                self.write(memory, addr, value)
            }
        }

        impl<const N: usize> DataWritable for [$ty; N] {
            type Writer = $array_writer<N>;

            fn writer(_layout: &impl MemoryLayout) -> Result<$array_writer<N>, DataError> {
                Ok($array_writer)
            }
        }
    };
}

prim_array_writable!(u8, U8Writer, U8ArrayWriter, IntType::U8.size());
prim_array_writable!(i8, I8Writer, I8ArrayWriter, IntType::S8.size());
prim_array_writable!(u16, U16Writer, U16ArrayWriter, IntType::U16.size());
prim_array_writable!(i16, I16Writer, I16ArrayWriter, IntType::S16.size());
prim_array_writable!(u32, U32Writer, U32ArrayWriter, IntType::U32.size());
prim_array_writable!(i32, I32Writer, I32ArrayWriter, IntType::S32.size());
prim_array_writable!(u64, U64Writer, U64ArrayWriter, IntType::U64.size());
prim_array_writable!(i64, I64Writer, I64ArrayWriter, IntType::S64.size());

prim_array_writable!(f32, F32Writer, F32ArrayWriter, FloatType::F32.size());
prim_array_writable!(f64, F64Writer, F64ArrayWriter, FloatType::F64.size());

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AddressArrayWriter<const N: usize>;

impl<const N: usize> AddressArrayWriter<N> {
    fn write<M: MemoryRead + MemoryWrite>(
        &self,
        memory: &mut M,
        addr: Address,
        value: [Address; N],
    ) -> Result<(), DataError> {
        let stride = memory.pointer_int_type().size();
        for i in 0..N {
            AddressWriter.write(memory, addr + i * stride, value[i])?;
        }
        Ok(())
    }
}

impl<const N: usize> DataWriter for AddressArrayWriter<N> {
    type Value = [Address; N];

    fn write<M: MemoryRead + MemoryWrite>(
        &self,
        memory: &mut M,
        addr: Address,
        value: [Address; N],
    ) -> Result<(), DataError> {
        self.write(memory, addr, value)
    }
}

impl<const N: usize> DataWritable for [Address; N] {
    type Writer = AddressArrayWriter<N>;

    fn writer(_layout: &impl MemoryLayout) -> Result<AddressArrayWriter<N>, DataError> {
        Ok(AddressArrayWriter)
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
        value: Value,
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
        value: Value,
    ) -> Result<(), DataError> {
        self.write(memory, addr, value)
    }
}

pub(crate) fn write_value_impl(
    memory: &mut impl MemoryWrite,
    address: Address,
    data_type: &DataTypeRef,
    value: Value,
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
                write_value_impl(
                    memory,
                    address + i * *stride,
                    base,
                    element.clone(),
                    concrete_types,
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
