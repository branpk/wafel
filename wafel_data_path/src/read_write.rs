#![allow(clippy::needless_range_loop)]

use wafel_data_type::{Address, FloatType, IntType, Namespace, TypeName};
use wafel_memory::MemoryRead;

use crate::{DataError, MemoryLayout};

// TODO: Arrays should determine stride based on the field type in derive?

pub trait DataReader {
    type Output;

    fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<Self::Output, DataError>;
}

pub trait DataReadable {
    type Reader: DataReader<Output = Self>;

    fn reader(layout: &impl MemoryLayout) -> Result<Self::Reader, DataError>;
}

pub type Reader<T> = <T as DataReadable>::Reader;

macro_rules! prim_readable {
    ($ty:ident, $reader:ident, $method:ident $(, $arg:expr)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
        pub struct $reader;

        impl DataReader for $reader {
            type Output = $ty;

            fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<$ty, DataError> {
                Ok(memory.$method(addr, $($arg),*)? as $ty)
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

        impl<const N: usize> DataReader for $array_reader<N> {
            type Output = [$ty; N];

            fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<[$ty; N], DataError> {
                let mut result = [Default::default(); N];
                let stride = $size;
                for i in 0..N {
                    result[i] = $reader.read(memory, addr + i * stride)?;
                }
                Ok(result)
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
