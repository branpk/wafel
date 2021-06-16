use std::{fmt, sync::Arc};

use wafel_data_type::{Address, DataType, DataTypeRef, IntValue, Value};
use wafel_layout::DataLayout;
use wafel_memory::{MemoryError, MemoryRead, MemoryWrite, SymbolLookup};

use crate::{
    compile,
    DataPathError::{self, *},
};

/// Internal representation of a global or local data path.
#[derive(Debug, Clone)]
pub(crate) struct DataPathImpl<R> {
    /// The original source for the data path.
    pub(crate) source: String,
    /// The root for the path (either a global variable address or a struct type).
    pub(crate) root: R,
    /// The operations to perform when evaluating the path.
    pub(crate) edges: Vec<DataPathEdge>,
    /// The mask to apply for an integer variable.
    pub(crate) mask: Option<IntValue>,
    /// The type of the value that the path points to.
    ///
    /// This should be "concrete", i.e. not a TypeName.
    pub(crate) concrete_type: DataTypeRef,
    /// A reference to the global DataLayout.
    pub(crate) layout: Arc<DataLayout>,
}

/// An operation that is applied when evaluating a data path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DataPathEdge {
    Offset(usize),
    Deref,
    Nullable,
}

/// A data path starting from a global variable address.
///
/// See module documentation for more information.
#[derive(Debug, Clone)]
pub struct GlobalDataPath(pub(crate) DataPathImpl<Address>);

/// A data path starting from a type, such as a specific struct.
///
/// See module documentation for more information.
#[derive(Debug, Clone)]
pub struct LocalDataPath(pub(crate) DataPathImpl<DataTypeRef>);

/// Either a global or a local data path.
#[derive(Debug, Clone)]
pub enum DataPath {
    /// A global data path.
    Global(GlobalDataPath),
    /// A local data path.
    Local(LocalDataPath),
}

impl GlobalDataPath {
    /// Compile a global data path from source.
    ///
    /// See module documentation for syntax.
    pub fn compile(
        layout: &Arc<DataLayout>,
        symbol_lookup: &impl SymbolLookup,
        source: &str,
    ) -> Result<Self, DataPathError> {
        compile::data_path(layout, symbol_lookup, source)?.into_global()
    }

    /// Get the source for the path.
    pub fn source(&self) -> &str {
        &self.0.source
    }

    /// Concatenate a global and local path.
    ///
    /// An error will be returned if the result type of `self` doesn't match the root type
    /// of `path`.
    pub fn concat(&self, path: &LocalDataPath) -> Result<Self, DataPathError> {
        concat_paths(&self.0, &path.0).map(Self)
    }

    /// Evaluate the path and return the address of the variable.
    ///
    /// Note that this will read from memory if the path passes through a pointer.
    ///
    /// None will only be returned if `?` is used in the data path.
    pub fn address(&self, memory: &impl MemoryRead) -> Result<Option<Address>, MemoryError> {
        self.address_impl(memory)
            .map_err(|error| MemoryError::Context {
                context: format!("while evaluating {}", self),
                error: Box::new(error),
            })
    }

    fn address_impl(&self, memory: &impl MemoryRead) -> Result<Option<Address>, MemoryError> {
        let mut address: Address = self.0.root;
        for edge in &self.0.edges {
            match edge {
                DataPathEdge::Offset(offset) => address = address + *offset,
                DataPathEdge::Deref => {
                    address = memory.read_address(address)?;
                }
                DataPathEdge::Nullable => {
                    if memory.read_address(address)?.is_null() {
                        return Ok(None);
                    }
                }
            }
        }
        Ok(Some(address))
    }

    /// Evaluate the path and return the value stored in the variable.
    pub fn read(&self, memory: &impl MemoryRead) -> Result<Value, MemoryError> {
        self.read_impl(memory)
            .map_err(|error| MemoryError::Context {
                context: format!("while reading {}", self),
                error: Box::new(error),
            })
    }

    fn read_impl(&self, memory: &impl MemoryRead) -> Result<Value, MemoryError> {
        match self.address_impl(memory)? {
            Some(address) => {
                let mut value = memory.read_value(address, &self.0.concrete_type, |type_name| {
                    self.0.layout.data_type(type_name).ok().cloned()
                })?;
                if let Some(mask) = self.0.mask {
                    let full_value = value.try_as_int().expect("mask on non-integer type");
                    value = (full_value & mask).into();
                }
                Ok(value)
            }
            None => Ok(Value::None),
        }
    }

    /// Evaluate the path and write `value` to the variable.
    pub fn write<M: MemoryRead + MemoryWrite>(
        &self,
        memory: &mut M,
        value: Value,
    ) -> Result<(), MemoryError> {
        self.write_impl(memory, value)
            .map_err(|error| MemoryError::Context {
                context: format!("while writing {}", self),
                error: Box::new(error),
            })
    }

    fn write_impl<M: MemoryRead + MemoryWrite>(
        &self,
        memory: &mut M,
        value: Value,
    ) -> Result<(), MemoryError> {
        match self.address_impl(memory)? {
            Some(address) => {
                match self.0.mask {
                    Some(mask) => {
                        let mask_value = value.try_as_int()?;
                        match self.concrete_type().as_ref() {
                            DataType::Int(int_type) => {
                                let mut full_value = memory.read_int(address, *int_type)?;
                                full_value &= !mask;
                                full_value |= mask_value & mask;
                                memory.write_int(address, *int_type, full_value)?;
                            }
                            _ => panic!("mask on non-integer type"),
                        }
                    }
                    None => {
                        memory.write_value(address, &self.0.concrete_type, value, |type_name| {
                            self.0.layout.data_type(type_name).ok().cloned()
                        })?;
                    }
                }
                Ok(())
            }
            None => Ok(()),
        }
    }

    /// Get the concrete data type that the path points to.
    pub fn concrete_type(&self) -> DataTypeRef {
        self.0.concrete_type.clone()
    }
}

impl LocalDataPath {
    /// Compile a local data path from source.
    ///
    /// See module documentation for syntax.
    pub fn compile(
        layout: &Arc<DataLayout>,
        symbol_lookup: &impl SymbolLookup,
        source: &str,
    ) -> Result<Self, DataPathError> {
        compile::data_path(layout, symbol_lookup, source)?.into_local()
    }

    /// Get the path's root data type.
    pub fn root_type(&self) -> DataTypeRef {
        self.0.root.clone()
    }

    /// Concatenate two local paths.
    ///
    /// An error will be returned if the result type of `self` doesn't match the root type
    /// of `path`.
    pub fn concat(&self, path: &LocalDataPath) -> Result<Self, DataPathError> {
        concat_paths(&self.0, &path.0).map(Self)
    }

    /// Get the concrete data type that the path points to.
    pub fn concrete_type(&self) -> DataTypeRef {
        self.0.concrete_type.clone()
    }

    /// Return the field offset for a path of the form `struct A.x`.
    pub fn field_offset(&self) -> Result<usize, DataPathError> {
        if self.0.edges.len() == 1 {
            if let Some(DataPathEdge::Offset(offset)) = self.0.edges.get(0) {
                return Ok(*offset);
            }
        }
        Err(NotAField {
            path: self.to_string(),
        })
    }
}

impl DataPath {
    /// Compile a data path from source.
    ///
    /// See module documentation for syntax.
    pub fn compile(
        layout: &Arc<DataLayout>,
        symbol_lookup: &impl SymbolLookup,
        source: &str,
    ) -> Result<Self, DataPathError> {
        compile::data_path(layout, symbol_lookup, source)
    }

    fn source(&self) -> &str {
        match self {
            Self::Global(path) => path.0.source.as_str(),
            Self::Local(path) => path.0.source.as_str(),
        }
    }

    /// Try to convert into a `GlobalDataPath`.
    pub fn into_global(self) -> Result<GlobalDataPath, DataPathError> {
        if let Self::Global(path) = self {
            Ok(path)
        } else {
            Err(ExpectedGlobalPath {
                path: self.source().to_owned(),
            })
        }
    }

    /// Try to convert into a `LocalDataPath`.
    pub fn into_local(self) -> Result<LocalDataPath, DataPathError> {
        if let Self::Local(path) = self {
            Ok(path)
        } else {
            Err(ExpectedLocalPath {
                path: self.source().to_owned(),
            })
        }
    }

    /// Get the concrete data type that the path points to.
    pub fn concrete_type(&self) -> DataTypeRef {
        match self {
            DataPath::Global(path) => path.concrete_type(),
            DataPath::Local(path) => path.concrete_type(),
        }
    }
}

fn concat_paths<R: Clone>(
    path1: &DataPathImpl<R>,
    path2: &DataPathImpl<DataTypeRef>,
) -> Result<DataPathImpl<R>, DataPathError> {
    if path1.concrete_type == path2.root {
        Ok(DataPathImpl {
            source: format!("{}+{}", path1.source, path2.source),
            root: path1.root.clone(),
            edges: path1
                .edges
                .iter()
                .chain(path2.edges.iter())
                .cloned()
                .collect(),
            mask: path2.mask,
            concrete_type: path2.concrete_type.clone(),
            layout: Arc::clone(&path1.layout),
        })
    } else {
        Err(ConcatTypeMismatch {
            path1: path1.source.to_owned(),
            type1: path1.concrete_type.clone(),
            path2: path2.source.to_owned(),
            type2: path2.concrete_type.clone(),
        })
    }
}

impl<R> fmt::Display for DataPathImpl<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.source)
    }
}

impl fmt::Display for GlobalDataPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for LocalDataPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for DataPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataPath::Global(path) => write!(f, "{}", path),
            DataPath::Local(path) => write!(f, "{}", path),
        }
    }
}