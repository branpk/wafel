use wafel_data_type::Address;
use wafel_memory::{MemoryRead, MemoryWrite};

use crate::{DataError, MemoryLayout};

/// A type that knows how to read a structured value from memory.
///
/// See [DataReadable].
pub trait DataReader {
    /// The type of value that is read from memory.
    type Value;

    /// Read the value from memory at the given address.
    fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<Self::Value, DataError>;
}

/// Trait for Rust types that can be read from memory.
pub trait DataReadable {
    /// The reader for the type.
    type Reader: DataReader<Value = Self>;

    /// Construct a reader using the given layout.
    ///
    /// This method is expected to do the heavy lifting for the read operation,
    /// like looking up struct field offsets.
    fn reader(layout: &impl MemoryLayout) -> Result<Self::Reader, DataError>;
}

/// Shorthand for the [Reader] of a [DataReadable].
pub type Reader<T> = <T as DataReadable>::Reader;

/// A type that knows how to write a structured value to memory.
///
/// See [DataWritable].
pub trait DataWriter {
    /// The type of value that is written to memory.
    type Value;

    /// Write the value to memory at the given address.
    fn write<M: MemoryRead + MemoryWrite>(
        &self,
        memory: &mut M,
        addr: Address,
        value: &Self::Value,
    ) -> Result<(), DataError>;
}

/// Trait for Rust types that can be written to memory.
pub trait DataWritable {
    /// The writer for the type.
    type Writer: DataWriter<Value = Self>;

    /// Construct a writer using the given layout.
    ///
    /// This method is expected to do the heavy lifting for the write operation,
    /// like looking up struct field offsets.
    fn writer(layout: &impl MemoryLayout) -> Result<Self::Writer, DataError>;
}

/// Shorthand for the [Writer] of a [DataWritable].
pub type Writer<T> = <T as DataWritable>::Writer;

/// A readable/writable type that can be used in an array since its stride is known.
pub trait DataStride {
    /// The stride of an array of this type of value.
    fn stride(layout: &impl MemoryLayout) -> Result<usize, DataError>;
}
