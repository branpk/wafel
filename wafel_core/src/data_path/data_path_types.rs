use super::{compile, DataPathErrorCause};
use crate::{
    error::Error,
    memory::{ClassifiedAddress, Memory},
};
use derive_more::Display;
use wafel_data_type::{Address, DataTypeRef, Value};

/// Internal representation of a global or local data path.
#[derive(Debug, Display, Clone)]
#[display(fmt = "{}", source)]
pub(super) struct DataPathImpl<R> {
    /// The original source for the data path.
    pub(super) source: String,
    /// The root for the path (either a global variable address or a struct type).
    pub(super) root: R,
    /// The operations to perform when evaluating the path.
    pub(super) edges: Vec<DataPathEdge>,
    /// The type of the value that the path points to.
    ///
    /// This should be "concrete", i.e. not a TypeName.
    pub(super) concrete_type: DataTypeRef,
}

/// An operation that is applied when evaluating a data path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DataPathEdge {
    Offset(usize),
    Deref,
    Nullable,
}

/// A data path starting from a global variable address.
///
/// See module documentation for more information.
#[derive(Debug, Display, Clone)]
pub struct GlobalDataPath(pub(super) DataPathImpl<Address>);

/// A data path starting from a type, such as a specific struct.
///
/// See module documentation for more information.
#[derive(Debug, Display, Clone)]
pub struct LocalDataPath(pub(super) DataPathImpl<DataTypeRef>);

/// Either a global or a local data path.
#[derive(Debug, Display, Clone)]
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
    pub fn compile(memory: &impl Memory, source: &str) -> Result<Self, Error> {
        compile::data_path(memory, source)?.into_global()
    }

    /// Get the source for the path.
    pub fn source(&self) -> &str {
        &self.0.source
    }

    /// Concatenate a global and local path.
    ///
    /// An error will be returned if the result type of `self` doesn't match the root type
    /// of `path`.
    pub fn concat(&self, path: &LocalDataPath) -> Result<Self, Error> {
        concat_paths(&self.0, &path.0).map(Self)
    }

    /// Evaluate the path and return the address of the variable.
    ///
    /// Note that this will read from memory if the path passes through a pointer.
    ///
    /// None will only be returned if `?` is used in the data path.
    pub fn address<M: Memory>(&self, memory: &M, slot: &M::Slot) -> Result<Option<Address>, Error> {
        self.address_impl(memory, slot)
            .map_err(|error| error.context(format!("path {}", self.0.source)))
    }

    fn address_impl<M: Memory>(
        &self,
        memory: &M,
        slot: &M::Slot,
    ) -> Result<Option<Address>, Error> {
        let mut address: Address = self.0.root;
        for edge in &self.0.edges {
            match edge {
                DataPathEdge::Offset(offset) => address = address + *offset,
                DataPathEdge::Deref => {
                    let classified = memory.classify_address(&address);
                    address = memory.read_address(slot, &classified)?;
                }
                DataPathEdge::Nullable => {
                    let classified = memory.classify_address(&address);
                    let address_value = memory.read_address(slot, &classified)?;

                    if let ClassifiedAddress::Invalid = memory.classify_address(&address_value) {
                        return Ok(None);
                    }
                }
            }
        }
        Ok(Some(address))
    }

    /// Evaluate the path and return the value stored in the variable.
    pub fn read<M: Memory>(&self, memory: &M, slot: &M::Slot) -> Result<Value, Error> {
        match self.address(memory, slot)? {
            Some(address) => memory
                .read_value(slot, &address, &self.0.concrete_type)
                .map_err(|error| error.context(format!("path {}", self.0.source))),
            None => Ok(Value::None),
        }
    }

    /// Evaluate the path and write `value` to the variable.
    pub fn write<M: Memory>(
        &self,
        memory: &M,
        slot: &mut M::Slot,
        value: &Value,
    ) -> Result<(), Error> {
        match self.address(memory, slot)? {
            Some(address) => memory
                .write_value(slot, &address, &self.0.concrete_type, value)
                .map_err(|error| error.context(format!("path {}", self.0.source))),
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
    pub fn compile(memory: &impl Memory, source: &str) -> Result<Self, Error> {
        compile::data_path(memory, source)?.into_local()
    }

    /// Get the path's root data type.
    pub fn root_type(&self) -> DataTypeRef {
        self.0.root.clone()
    }

    /// Concatenate two local paths.
    ///
    /// An error will be returned if the result type of `self` doesn't match the root type
    /// of `path`.
    pub fn concat(&self, path: &LocalDataPath) -> Result<Self, Error> {
        concat_paths(&self.0, &path.0).map(Self)
    }

    /// Get the concrete data type that the path points to.
    pub fn concrete_type(&self) -> DataTypeRef {
        self.0.concrete_type.clone()
    }

    /// Return the field offset for a path of the form `struct A.x`.
    pub fn field_offset(&self) -> Result<usize, Error> {
        if self.0.edges.len() == 1 {
            if let Some(DataPathEdge::Offset(offset)) = self.0.edges.get(0) {
                return Ok(*offset);
            }
        }
        Err(DataPathErrorCause::NotAField {
            path: self.to_string(),
        }
        .into())
    }
}

impl DataPath {
    /// Compile a data path from source.
    ///
    /// See module documentation for syntax.
    pub fn compile(memory: &impl Memory, source: &str) -> Result<Self, Error> {
        compile::data_path(memory, source)
    }

    fn source(&self) -> &str {
        match self {
            Self::Global(path) => path.0.source.as_str(),
            Self::Local(path) => path.0.source.as_str(),
        }
    }

    /// Try to convert into a `GlobalDataPath`.
    pub fn into_global(self) -> Result<GlobalDataPath, Error> {
        if let Self::Global(path) = self {
            Ok(path)
        } else {
            Err(DataPathErrorCause::ExpectedGlobalPath {
                path: self.source().to_owned(),
            }
            .into())
        }
    }

    /// Try to convert into a `LocalDataPath`.
    pub fn into_local(self) -> Result<LocalDataPath, Error> {
        if let Self::Local(path) = self {
            Ok(path)
        } else {
            Err(DataPathErrorCause::ExpectedLocalPath {
                path: self.source().to_owned(),
            }
            .into())
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
) -> Result<DataPathImpl<R>, Error> {
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
            concrete_type: path2.concrete_type.clone(),
        })
    } else {
        Err(DataPathErrorCause::DataPathConcatTypeMismatch {
            path1: path1.source.to_owned(),
            type1: path1.concrete_type.clone(),
            path2: path2.source.to_owned(),
            type2: path2.concrete_type.clone(),
        }
        .into())
    }
}
