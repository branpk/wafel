//! Types and functions for representing C data types.

use std::{collections::HashMap, error::Error, fmt, sync::Arc};

/// A representation of a C data type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataType {
    /// Void, typically used as a pointer target or function return type.
    Void,
    /// An integer type.
    Int(IntType),
    /// A float type.
    Float(FloatType),
    /// A pointer type.
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
    Struct {
        /// The fields contained in the struct.
        ///
        /// Anonymous fields should be given a name on construction, typically `__anon`.
        fields: HashMap<String, Field>,
    },
    /// A union type.
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
pub type DataTypeRef = Arc<DataType>;

/// Integer types of different sizes and signedness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntType {
    /// 8 bit unsigned int
    U8,
    /// 8 bit signed int
    S8,
    /// 16 bit unsigned int
    U16,
    /// 16 bit signed int
    S16,
    /// 32 bit unsigned int
    U32,
    /// 32 bit signed int
    S32,
    /// 64 bit unsigned int
    U64,
    /// 64 bit signed int
    S64,
}

/// Float types of different sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatType {
    /// 32 bit float
    F32,
    /// 64 bit float
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Namespace {
    /// Types defined using `struct A { ... }`.
    Struct,
    /// Types defined using `union A { ... }`.
    Union,
    /// Types defined using `typedef ... A`.
    Typedef,
}

/// A symbolic reference to a type definition, e.g. `struct Foo`.
///
/// In C, `struct A` can refer to a different type than `union A` or `A`, so we need to record
/// both the "namespace" (`struct`, `union`, or `typedef`) as well as the raw name (`A`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
        matches!(self, Self::Void)
    }

    /// Return true if the data type is an integer type.
    pub fn is_int(&self) -> bool {
        matches!(self, Self::Int(_))
    }

    /// Return true if the data type is a float type.
    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(_))
    }

    /// Return true if the data type is a pointer type.
    pub fn is_pointer(&self) -> bool {
        matches!(self, Self::Pointer { .. })
    }

    /// Return true if the data type is an array type.
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array { .. })
    }

    /// Return true if the data type is a struct type.
    pub fn is_struct(&self) -> bool {
        matches!(self, Self::Struct { .. })
    }

    /// Return true if the data type is a union type.
    pub fn is_union(&self) -> bool {
        matches!(self, Self::Union { .. })
    }

    /// Return true if the data type is a type name.
    pub fn is_name(&self) -> bool {
        matches!(self, Self::Name(_))
    }

    /// Return the stride for an array or pointer type.
    pub fn stride(&self) -> Result<Option<usize>, NotAnArrayOrPointer> {
        match self {
            DataType::Pointer { stride, .. } => Ok(*stride),
            DataType::Array { stride, .. } => Ok(Some(*stride)),
            _ => Err(NotAnArrayOrPointer),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NotAnArrayOrPointer;

impl fmt::Display for NotAnArrayOrPointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DataType::stride called a non-array/pointer type")
    }
}

impl Error for NotAnArrayOrPointer {}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataType::Void => write!(f, "void"),
            DataType::Int(int_type) => write!(f, "{}", int_type),
            DataType::Float(float_type) => write!(f, "{}", float_type),
            DataType::Pointer { base, stride: _ } => write!(f, "ptr[{}]", base),
            DataType::Array {
                base,
                length,
                stride: _,
            } => match length {
                Some(length) => write!(f, "array[{}; {}]", base, length),
                None => write!(f, "array[{}]", base),
            },
            DataType::Struct { fields } => {
                write!(f, "struct ")?;
                display_fields(f, fields)?;
                Ok(())
            }
            DataType::Union { fields } => {
                write!(f, "union ")?;
                display_fields(f, fields)?;
                Ok(())
            }
            DataType::Name(name) => write!(f, "{}", name),
        }
    }
}

fn display_fields(f: &mut fmt::Formatter<'_>, fields: &HashMap<String, Field>) -> fmt::Result {
    let mut sorted_fields = fields.iter().collect::<Vec<_>>();
    sorted_fields.sort_by_key(|(_, field)| field.offset);
    writeln!(f, "{{")?;
    for (name, field) in sorted_fields {
        writeln!(
            f,
            "  {}: {}",
            name,
            format!("{}", field.data_type).replace("\n", "\n  ")
        )?;
    }
    write!(f, "}}")?;
    Ok(())
}

impl fmt::Display for IntType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntType::U8 => write!(f, "u8"),
            IntType::S8 => write!(f, "s8"),
            IntType::U16 => write!(f, "u16"),
            IntType::S16 => write!(f, "s16"),
            IntType::U32 => write!(f, "u32"),
            IntType::S32 => write!(f, "s32"),
            IntType::U64 => write!(f, "u64"),
            IntType::S64 => write!(f, "s64"),
        }
    }
}

impl fmt::Display for FloatType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FloatType::F32 => write!(f, "f32"),
            FloatType::F64 => write!(f, "f64"),
        }
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Namespace::Struct => write!(f, "struct"),
            Namespace::Union => write!(f, "union"),
            Namespace::Typedef => write!(f, "typedef"),
        }
    }
}

impl fmt::Display for TypeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.namespace, self.name)
    }
}
