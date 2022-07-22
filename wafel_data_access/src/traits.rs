use wafel_data_type::Address;
use wafel_memory::MemoryRead;

use crate::{DataError, MemoryLayout};

/// A type that knows how to read a structured value from memory.
///
/// See [DataReadable].
pub trait DataReader {
    /// The type of value that is read from memory.
    type Output;

    /// Read the value from memory at the given address.
    fn read(&self, memory: &impl MemoryRead, addr: Address) -> Result<Self::Output, DataError>;
}

/// Trait for Rust types that can be read from memory.
pub trait DataReadable {
    /// The reader for the type.
    type Reader: DataReader<Output = Self>;

    /// Construct a reader using the given layout.
    ///
    /// This method is expected to do the heavy lifting for the read operation,
    /// like looking up struct field offsets=.
    fn reader(layout: &impl MemoryLayout) -> Result<Self::Reader, DataError>;
}

/// Shorthand for the [Reader] of a [DataReadable].
pub type Reader<T> = <T as DataReadable>::Reader;
