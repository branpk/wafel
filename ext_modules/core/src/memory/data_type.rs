//! Types and functions for representing C data types.

use derive_more::Display;
use itertools::Itertools;
use std::{collections::HashMap, fmt::Debug, hash::Hash, sync::Arc};
use textwrap::indent;

/// A representation of a C data type.
#[derive(Debug, Display, Clone, PartialEq, Eq)]
pub enum DataType {
    /// Void, typically used as a pointer target or function return type.
    #[display(fmt = "void")]
    Void,
    /// An integer type.
    Int(IntType),
    /// A float type.
    Float(FloatType),
    /// A pointer type.
    #[display(fmt = "ptr[{}]", base)]
    Pointer {
        /// The type being pointed to.
        base: DataTypeRef,
        /// The size of the type being pointed to, if known.
        ///
        /// This is used when applying a subscript to a pointer, e.g. `p[3]`. It is otherwise
        /// not necessary.
        stride: Option<usize>,
    },
    /// An array type, optionally with a length.
    #[display(fmt = "{}", "display_array(base, *length)")]
    Array {
        /// The element type.
        base: DataTypeRef,
        /// The length of the array, if known.
        length: Option<usize>,
        /// The size of the element type.
        ///
        /// This is used when indexing into the array.
        stride: usize,
    },
    /// A struct type.
    #[display(fmt = "{}", r#"display_struct_or_union("struct", fields)"#)]
    Struct {
        /// The fields contained in the struct.
        ///
        /// Anonymous fields should be given a name on construction, typically `__anon`.
        fields: HashMap<String, Field>,
    },
    /// A union type.
    #[display(fmt = "{}", r#"display_struct_or_union("union", fields)"#)]
    Union {
        /// The fields contained in the union.
        ///
        /// Anonymous fields should be given a name on construction, typically `__anon`.
        fields: HashMap<String, Field>,
    },
    /// A symbolic reference to a type definition, e.g. `struct Foo`.
    Name(TypeName),
}

/// A reference to a `DataType`.
///
/// This should typically be used instead of `DataType` for more efficient `Clone` and `Eq`.
pub type DataTypeRef = Arc<DataType>;

fn display_array(base: &DataTypeRef, length: Option<usize>) -> String {
    match length {
        Some(length) => format!("array[{}; {}]", base, length),
        None => format!("array[{}]", base),
    }
}

fn display_struct_or_union(kind: &str, fields: &HashMap<String, Field>) -> String {
    let fields_str: String = fields
        .iter()
        .sorted_by_key(|(_, field)| field.offset)
        .map(|(name, field)| indent(format!("{}: {}\n", name, field.data_type).as_ref(), "  "))
        .collect();
    format!("{} {{\n{}}}", kind, fields_str)
}

/// Integer types of different sizes and signedness.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntType {
    /// 8 bit unsigned int
    #[display(fmt = "u8")]
    U8,
    /// 8 bit signed int
    #[display(fmt = "s8")]
    S8,
    /// 16 bit unsigned int
    #[display(fmt = "u16")]
    U16,
    /// 16 bit signed int
    #[display(fmt = "s16")]
    S16,
    /// 32 bit unsigned int
    #[display(fmt = "u32")]
    U32,
    /// 32 bit signed int
    #[display(fmt = "s32")]
    S32,
    /// 64 bit unsigned int
    #[display(fmt = "u64")]
    U64,
    /// 64 bit signed int
    #[display(fmt = "s64")]
    S64,
}

/// Float types of different sizes.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatType {
    /// 32 bit float
    #[display(fmt = "f32")]
    F32,
    /// 64 bit float
    #[display(fmt = "f64")]
    F64,
}

impl IntType {
    /// The size of the int in bytes.
    pub fn size(&self) -> usize {
        match self {
            Self::U8 => 1,
            Self::S8 => 1,
            Self::U16 => 2,
            Self::S16 => 2,
            Self::U32 => 4,
            Self::S32 => 4,
            Self::U64 => 8,
            Self::S64 => 8,
        }
    }
}

impl FloatType {
    /// The size of the float in bytes.
    pub fn size(&self) -> usize {
        match self {
            Self::F32 => 4,
            Self::F64 => 8,
        }
    }
}

/// The C type namespaces.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Namespace {
    /// Types defined using `struct A { ... }`.
    #[display(fmt = "struct")]
    Struct,
    /// Types defined using `union A { ... }`.
    #[display(fmt = "union")]
    Union,
    /// Types defined using `typedef ... A`.
    #[display(fmt = "typedef")]
    Typedef,
}

/// A symbolic reference to a type definition, e.g. `struct Foo`.
///
/// In C, `struct A` can refer to a different type than `union A` or `A`, so we need to record
/// both the "namespace" (`struct`, `union`, or `typedef`) as well as the raw name (`A`).
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
#[display(fmt = "{} {}", namespace, name)]
pub struct TypeName {
    /// The namespace that the type name blongs to.
    pub namespace: Namespace,
    /// The raw name of the type.
    pub name: String,
}

/// A field in a struct or union.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// The byte offset within the struct or union.
    pub offset: usize,
    /// The type of the field.
    pub data_type: DataTypeRef,
}

impl DataType {
    /// Return true if the data type is void.
    pub fn is_void(&self) -> bool {
        if let Self::Void = self {
            true
        } else {
            false
        }
    }

    /// Return true if the data type is an integer type.
    pub fn is_int(&self) -> bool {
        if let Self::Int(_) = self {
            true
        } else {
            false
        }
    }

    /// Return true if the data type is a float type.
    pub fn is_float(&self) -> bool {
        if let Self::Float(_) = self {
            true
        } else {
            false
        }
    }

    /// Return true if the data type is a pointer type.
    pub fn is_pointer(&self) -> bool {
        if let Self::Pointer { .. } = self {
            true
        } else {
            false
        }
    }

    /// Return true if the data type is an array type.
    pub fn is_array(&self) -> bool {
        if let Self::Array { .. } = self {
            true
        } else {
            false
        }
    }

    /// Return true if the data type is a struct type.
    pub fn is_struct(&self) -> bool {
        if let Self::Struct { .. } = self {
            true
        } else {
            false
        }
    }

    /// Return true if the data type is a union type.
    pub fn is_union(&self) -> bool {
        if let Self::Union { .. } = self {
            true
        } else {
            false
        }
    }

    /// Return true if the data type is a type name.
    pub fn is_name(&self) -> bool {
        if let Self::Name(_) = self {
            true
        } else {
            false
        }
    }
}
