//! Types and functions for representing C data types.

use std::{fmt, mem, sync::Arc};

use indexmap::IndexMap;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

use crate::error::DataTypeError;

/// A representation of a C data type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
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
        fields: IndexMap<String, Field>,
    },
    /// A union type.
    Union {
        /// The fields contained in the union.
        ///
        /// Anonymous fields should be given a name on construction, typically `__anon`.
        fields: IndexMap<String, Field>,
    },
    /// A symbolic reference to a type definition, e.g. `struct Foo`.
    Name(TypeName),
}

/// A reference to a `DataType`.
pub type DataTypeRef = Arc<DataType>;

/// Integer types of different sizes and signedness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FloatType {
    /// 32 bit float
    F32,
    /// 64 bit float
    F64,
}

impl IntType {
    /// An unsigned IntType with the same size as native usize.
    pub fn u_ptr_native() -> Self {
        match mem::size_of::<usize>() {
            4 => Self::U32,
            8 => Self::U64,
            s => unimplemented!("size_of<usize> = {}", s),
        }
    }

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

    /// Returns an unsigned int type with the given size in bytes.
    pub fn unsigned_with_size(size: usize) -> Self {
        match size {
            1 => Self::U8,
            2 => Self::U16,
            4 => Self::U32,
            8 => Self::U64,
            _ => unimplemented!("unsigned int with size {}", size),
        }
    }

    /// Returns a signed int type with the given size in bytes.
    pub fn signed_with_size(size: usize) -> Self {
        match size {
            1 => Self::S8,
            2 => Self::S16,
            4 => Self::S32,
            8 => Self::S64,
            _ => unimplemented!("signed int with size {}", size),
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

impl TypeName {
    /// Return a struct type name.
    pub fn of_struct(name: &str) -> Self {
        Self {
            namespace: Namespace::Struct,
            name: name.to_string(),
        }
    }

    /// Return a union type name.
    pub fn of_union(name: &str) -> Self {
        Self {
            namespace: Namespace::Union,
            name: name.to_string(),
        }
    }

    /// Return a typedef type name.
    pub fn of_typedef(name: &str) -> Self {
        Self {
            namespace: Namespace::Typedef,
            name: name.to_string(),
        }
    }
}

/// A field in a struct or union.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Return an error if the data type is not void.
    pub fn try_as_void(&self) -> Result<(), DataTypeError> {
        if self.is_void() {
            Ok(())
        } else {
            Err(DataTypeError::ExpectedType {
                expected: "void".into(),
                actual: self.clone(),
            })
        }
    }

    /// Return true if the data type is an integer type.
    pub fn is_int(&self) -> bool {
        matches!(self, Self::Int(_))
    }

    /// Convert the data type to an int type.
    pub fn try_as_int(&self) -> Result<IntType, DataTypeError> {
        if let Self::Int(int_type) = self {
            Ok(*int_type)
        } else {
            Err(DataTypeError::ExpectedType {
                expected: "int type".into(),
                actual: self.clone(),
            })
        }
    }

    /// Return true if the data type is a float type.
    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(_))
    }

    /// Convert the data type to a float type.
    pub fn try_as_float(&self) -> Result<FloatType, DataTypeError> {
        if let Self::Float(float_type) = self {
            Ok(*float_type)
        } else {
            Err(DataTypeError::ExpectedType {
                expected: "float type".into(),
                actual: self.clone(),
            })
        }
    }

    /// Return true if the data type is a pointer type.
    pub fn is_pointer(&self) -> bool {
        matches!(self, Self::Pointer { .. })
    }

    /// Convert the data type to a pointer type.
    pub fn try_as_pointer(&self) -> Result<(&DataTypeRef, Option<usize>), DataTypeError> {
        if let Self::Pointer { base, stride } = self {
            Ok((base, *stride))
        } else {
            Err(DataTypeError::ExpectedType {
                expected: "pointer type".into(),
                actual: self.clone(),
            })
        }
    }

    /// Return true if the data type is an array type.
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array { .. })
    }

    /// Convert the data type to an array type.
    pub fn try_as_array(&self) -> Result<(&DataTypeRef, Option<usize>, usize), DataTypeError> {
        if let Self::Array {
            base,
            length,
            stride,
        } = self
        {
            Ok((base, *length, *stride))
        } else {
            Err(DataTypeError::ExpectedType {
                expected: "array type".into(),
                actual: self.clone(),
            })
        }
    }

    /// Return true if the data type is a struct type.
    pub fn is_struct(&self) -> bool {
        matches!(self, Self::Struct { .. })
    }

    /// Convert the data type to a struct type.
    pub fn try_as_struct(&self) -> Result<&IndexMap<String, Field>, DataTypeError> {
        if let Self::Struct { fields } = self {
            Ok(fields)
        } else {
            Err(DataTypeError::ExpectedType {
                expected: "struct type".into(),
                actual: self.clone(),
            })
        }
    }

    /// Return true if the data type is a union type.
    pub fn is_union(&self) -> bool {
        matches!(self, Self::Union { .. })
    }

    /// Convert the data type to a union type.
    pub fn try_as_union(&self) -> Result<&IndexMap<String, Field>, DataTypeError> {
        if let Self::Union { fields } = self {
            Ok(fields)
        } else {
            Err(DataTypeError::ExpectedType {
                expected: "union type".into(),
                actual: self.clone(),
            })
        }
    }

    /// Return true if the data type is a type name.
    pub fn is_name(&self) -> bool {
        matches!(self, Self::Name(_))
    }

    /// Convert the data type to a type name.
    pub fn try_as_name(&self) -> Result<&TypeName, DataTypeError> {
        if let Self::Name(name) = self {
            Ok(name)
        } else {
            Err(DataTypeError::ExpectedType {
                expected: "type name".into(),
                actual: self.clone(),
            })
        }
    }

    /// Return the stride for an array or pointer type.
    pub fn stride(&self) -> Result<Option<usize>, DataTypeError> {
        match self {
            DataType::Pointer { stride, .. } => Ok(*stride),
            DataType::Array { stride, .. } => Ok(Some(*stride)),
            _ => Err(DataTypeError::ExpectedType {
                expected: "pointer or array".into(),
                actual: self.clone(),
            }),
        }
    }

    /// Look up a field by name in a struct type.
    pub fn struct_field(&self, name: &str) -> Result<&Field, DataTypeError> {
        let fields = self.try_as_struct()?;
        match fields.get(name) {
            Some(field) => Ok(field),
            None => Err(DataTypeError::NoSuchField {
                name: name.to_string(),
            }),
        }
    }
}

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

fn display_fields(f: &mut fmt::Formatter<'_>, fields: &IndexMap<String, Field>) -> fmt::Result {
    writeln!(f, "{{")?;
    for (name, field) in fields {
        writeln!(
            f,
            "  {}: {}",
            name,
            format!("{}", field.data_type).replace('\n', "\n  ")
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
        match self.namespace {
            Namespace::Struct => write!(f, "struct {}", self.name),
            Namespace::Union => write!(f, "union {}", self.name),
            Namespace::Typedef => write!(f, "{}", self.name),
        }
    }
}

impl Serialize for TypeName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{} {}", self.namespace, self.name))
    }
}

impl<'de> Deserialize<'de> for TypeName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        let (namespace_str, name) = s
            .split_once(' ')
            .ok_or_else(|| D::Error::custom("type name must have form '<namespace> <name>'"))?;
        let namespace = match namespace_str {
            "struct" => Namespace::Struct,
            "union" => Namespace::Union,
            "typedef" => Namespace::Typedef,
            _ => {
                return Err(D::Error::custom(&format!(
                    "invalid namespace: {}",
                    namespace_str
                )))
            }
        };
        Ok(Self {
            namespace,
            name: name.to_string(),
        })
    }
}
