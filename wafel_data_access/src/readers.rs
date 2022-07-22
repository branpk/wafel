#![allow(clippy::needless_range_loop)]
#![allow(missing_docs)]

use indexmap::IndexMap;
use wafel_data_type::{Address, DataType, DataTypeRef, FloatType, IntType, TypeName, Value};
use wafel_memory::MemoryRead;

use crate::{
    DataError::{self, *},
    DataReadable, DataReader, MemoryLayout,
};

// TODO: Arrays should determine stride based on the field type in derive?

macro_rules! prim_readable {
    ($ty:ident, $reader:ident, $method:ident $(, $arg:expr)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
        pub struct $reader;

        impl $reader {
            pub fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<$ty, DataError> {
                Ok(memory.$method(addr, $($arg),*)? as $ty)
            }
        }

        impl DataReader for $reader {
            type Output = $ty;

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
    };
}

prim_readable!(u8, U8Reader, read_int, IntType::U8);
prim_readable!(i8, I8Reader, read_int, IntType::S8);
prim_readable!(u16, U16Reader, read_int, IntType::U16);
prim_readable!(i16, I16Reader, read_int, IntType::S16);
prim_readable!(u32, U32Reader, read_int, IntType::U32);
prim_readable!(i32, I32Reader, read_int, IntType::S32);
prim_readable!(u64, U64Reader, read_int, IntType::U64);
prim_readable!(i64, I64Reader, read_int, IntType::S64);

prim_readable!(f32, F32Reader, read_float, FloatType::F32);
prim_readable!(f64, F64Reader, read_float, FloatType::F64);

prim_readable!(Address, AddressReader, read_address);

macro_rules! prim_array_readable {
    ($ty:ident, $reader:ident, $array_reader:ident, $size:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
        pub struct $array_reader<const N: usize>;

        impl<const N: usize> $array_reader<N> {
            pub fn read(
                &self,
                memory: &impl MemoryRead,
                addr: Address,
            ) -> Result<[$ty; N], DataError> {
                let mut result = [Default::default(); N];
                let stride = $size;
                for i in 0..N {
                    result[i] = $reader.read(memory, addr + i * stride)?;
                }
                Ok(result)
            }
        }

        impl<const N: usize> DataReader for $array_reader<N> {
            type Output = [$ty; N];

            fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<[$ty; N], DataError> {
                self.read(memory, addr)
            }
        }

        impl<const N: usize> DataReadable for [$ty; N] {
            type Reader = $array_reader<N>;

            fn reader(_layout: &impl MemoryLayout) -> Result<$array_reader<N>, DataError> {
                Ok($array_reader)
            }
        }
    };
}

prim_array_readable!(u8, U8Reader, U8ArrayReader, IntType::U8.size());
prim_array_readable!(i8, I8Reader, I8ArrayReader, IntType::S8.size());
prim_array_readable!(u16, U16Reader, U16ArrayReader, IntType::U16.size());
prim_array_readable!(i16, I16Reader, I16ArrayReader, IntType::S16.size());
prim_array_readable!(u32, U32Reader, U32ArrayReader, IntType::U32.size());
prim_array_readable!(i32, I32Reader, I32ArrayReader, IntType::S32.size());
prim_array_readable!(u64, U64Reader, U64ArrayReader, IntType::U64.size());
prim_array_readable!(i64, I64Reader, I64ArrayReader, IntType::S64.size());

prim_array_readable!(f32, F32Reader, F32ArrayReader, FloatType::F32.size());
prim_array_readable!(f64, F64Reader, F64ArrayReader, FloatType::F64.size());

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AddressArrayReader<const N: usize>;

impl<const N: usize> DataReader for AddressArrayReader<N> {
    type Output = [Address; N];
    fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<[Address; N], DataError> {
        let mut result = [Default::default(); N];
        let stride = memory.pointer_int_type().size();
        for i in 0..N {
            result[i] = AddressReader.read(memory, addr + i * stride)?;
        }
        Ok(result)
    }
}
impl<const N: usize> DataReadable for [Address; N] {
    type Reader = AddressArrayReader<N>;
    fn reader(_layout: &impl MemoryLayout) -> Result<AddressArrayReader<N>, DataError> {
        Ok(AddressArrayReader)
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
    type Output = Value;

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
        DataType::Pointer { .. } => Value::Address(memory.read_address(addr)?),
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
