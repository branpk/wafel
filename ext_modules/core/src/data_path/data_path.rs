use super::{compile, DataPathErrorCause};
use crate::{
    error::Error,
    memory::{data_type::DataTypeRef, Memory, Value},
};
use derivative::Derivative;
use derive_more::Display;
use std::{
    borrow::Borrow,
    fmt::{self, Display},
};

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
}

/// A data path starting from a global variable address.
///
/// See module documentation for more information.
#[derive(Debug, Derivative)]
#[derivative(Clone(bound = "M::Address: Clone"))]
pub struct GlobalDataPath<M: Memory>(pub(super) DataPathImpl<M::Address>);

/// A data path starting from a type, such as a specific struct.
///
/// See module documentation for more information.
#[derive(Debug, Display, Clone)]
pub struct LocalDataPath(pub(super) DataPathImpl<DataTypeRef>);

/// Either a global or a local data path.
#[derive(Debug, Display, Derivative)]
#[derivative(Clone(bound = "M::Address: Clone"))]
pub enum DataPath<M: Memory> {
    /// A global data path.
    Global(GlobalDataPath<M>),
    /// A local data path.
    Local(LocalDataPath),
}

impl<M: Memory> GlobalDataPath<M> {
    /// Compile a global data path from source.
    ///
    /// See module documentation for syntax.
    pub fn compile(memory: &M, source: &str) -> Result<Self, Error> {
        Ok(compile::data_path(memory, source)?.into_global()?)
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
    pub fn address(&self, memory: &M, slot: &M::Slot) -> Result<M::Address, Error> {
        let result: Result<_, Error> = try {
            let mut address = self.0.root.clone();
            for edge in &self.0.edges {
                match edge {
                    DataPathEdge::Offset(offset) => address = address + *offset,
                    DataPathEdge::Deref => {
                        let classified = memory.classify_address(&address)?;
                        address = memory.read_address(slot, &classified)?;
                    }
                }
            }
            address
        };
        result.map_err(|error| error.context(format!("path {}", self.0.source)))
    }

    /// Evaluate the path and return the value stored in the variable.
    pub fn read(&self, memory: &M, slot: &M::Slot) -> Result<Value<M::Address>, Error> {
        let address = self.address(memory, slot)?;
        memory
            .read_value(slot, &address, &self.0.concrete_type)
            .map_err(|error| error.context(format!("path {}", self.0.source)))
    }

    /// Evaluate the path and write `value` to the variable.
    pub fn write(
        &self,
        memory: &M,
        slot: &mut M::Slot,
        value: &Value<M::Address>,
    ) -> Result<(), Error> {
        let address = self.address(memory, slot)?;
        memory
            .write_value(slot, &address, &self.0.concrete_type, value)
            .map_err(|error| error.context(format!("path {}", self.0.source)))
    }

    /// Get the concrete data type that the path points to.
    pub fn concrete_type(&self) -> DataTypeRef {
        self.0.concrete_type.clone()
    }
}

impl<M: Memory> Display for GlobalDataPath<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl LocalDataPath {
    /// Compile a local data path from source.
    ///
    /// See module documentation for syntax.
    pub fn compile<M: Memory>(memory: &M, source: &str) -> Result<Self, Error> {
        Ok(compile::data_path(memory, source)?.into_local()?)
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
}

impl<M: Memory> DataPath<M> {
    /// Compile a data path from source.
    ///
    /// See module documentation for syntax.
    pub fn compile(memory: &M, source: &str) -> Result<Self, Error> {
        compile::data_path(memory, source)
    }

    fn source(&self) -> &str {
        match self {
            Self::Global(path) => path.0.source.as_str(),
            Self::Local(path) => path.0.source.as_str(),
        }
    }

    /// Try to convert into a `GlobalDataPath`.
    pub fn into_global(self) -> Result<GlobalDataPath<M>, Error> {
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

/// A trait for objects that can be used as global data paths.
///
/// This allows `GlobalDataPath`s and strings to be used as paths.
pub trait AsGlobalDataPath<M: Memory> {
    /// The reference type.
    type PathRef: Borrow<GlobalDataPath<M>>;

    /// Perform the conversion.
    fn as_global_data_path(&self, memory: &M) -> Result<Self::PathRef, Error>;
}

impl<'a, M: Memory> AsGlobalDataPath<M> for &'a GlobalDataPath<M> {
    type PathRef = &'a GlobalDataPath<M>;

    fn as_global_data_path(&self, _memory: &M) -> Result<Self::PathRef, Error> {
        Ok(self)
    }
}

impl<M: Memory, S: AsRef<str>> AsGlobalDataPath<M> for S {
    type PathRef = GlobalDataPath<M>;

    fn as_global_data_path(&self, memory: &M) -> Result<Self::PathRef, Error> {
        memory.global_path(self.as_ref())
    }
}
