//! Recording and looking up type and global variable definitions.

use std::{collections::HashMap, fmt};

use wafel_data_type::{DataType, DataTypeRef, IntValue, TypeName};

/// A description of accessible variables and types.
#[derive(Debug, Clone, Default)]
pub struct DataLayout {
    /// The definitions of structs, unions, and typedefs.
    pub type_defns: HashMap<TypeName, DataTypeRef>,
    /// The types of global variables and functions.
    pub globals: HashMap<String, DataTypeRef>,
    /// The values of integer constants.
    pub constants: HashMap<String, Constant>,
}

/// A constant's value and source.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Constant {
    /// The integer value for the constant.
    pub value: IntValue,
    /// The source for the constant.
    pub source: ConstantSource,
}

/// The source for a constant value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstantSource {
    /// The constant is defined as an enum variant.
    Enum {
        /// The name of the enum, or None for an anonymous enum.
        name: Option<String>,
    },
    /// The constant is defined as a macro.
    Macro,
}

impl DataLayout {
    /// Create an empty data layout.
    pub fn new() -> Self {
        Self {
            type_defns: HashMap::new(),
            globals: HashMap::new(),
            constants: HashMap::new(),
        }
    }

    /// Look up the definition of a type name.
    pub fn data_type(&self, name: &TypeName) -> Option<&DataTypeRef> {
        self.type_defns.get(name)
    }

    /// Look up the definition of a type name.
    ///
    /// This returns a mutable reference to the DataTypeRef. This is only useful if
    /// the data type hasn't been used in multiple places.
    pub fn data_type_mut(&mut self, name: &TypeName) -> Option<&mut DataTypeRef> {
        self.type_defns.get_mut(name)
    }

    /// Recursively look up a type name.
    pub fn concrete_type(&self, data_type: &DataTypeRef) -> Option<DataTypeRef> {
        let mut data_type = data_type.clone();
        while let DataType::Name(name) = data_type.as_ref() {
            data_type = self.data_type(name)?.clone();
        }
        Some(data_type)
    }

    /// Look up the type of a global variable.
    pub fn global(&self, name: &str) -> Option<&DataTypeRef> {
        self.globals.get(name)
    }

    /// Look up the value of a constant.
    pub fn constant(&self, name: &str) -> Option<&Constant> {
        self.constants.get(name)
    }
}

impl fmt::Display for DataLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (name, data_type) in &self.type_defns {
            writeln!(f, "{} = {}", name, data_type)?;
        }
        for (name, data_type) in &self.globals {
            writeln!(f, "{}: {}", name, data_type)?;
        }
        for (name, value) in &self.constants {
            writeln!(f, "{} := {}", name, value)?;
        }
        Ok(())
    }
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.value, self.source)
    }
}

impl fmt::Display for ConstantSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstantSource::Enum { name } => match name {
                Some(name) => write!(f, "enum {}", name),
                None => write!(f, "anonymous enum"),
            },
            ConstantSource::Macro => write!(f, "macro"),
        }
    }
}
