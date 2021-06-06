//! Debugging and structural information extracted from a DLL.

use super::{LayoutError, LayoutErrorCause};
use crate::memory::{Constant, ConstantSource, DataLayout, IntValue};
use derive_more::Display;
use gimli::{
    AttributeValue, DebuggingInformationEntry, DwAt, Dwarf, EndianSlice, EntriesTree,
    EntriesTreeNode, Reader, RunTimeEndian, SectionId, Unit,
};
use object::{Object, ObjectSection, ObjectSegment};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::{self, Display},
    fs, iter,
    path::Path,
};
use wafel_types::{
    shallow::{
        build_data_types, get_size_from_pre_types, BuildDataTypesError, PreDataType,
        PreDataTypeSize, ShallowDataType, ShallowField,
    },
    FloatType, IntType, Namespace, TypeName,
};

/// Debugging and structural information extracted from a DLL.
#[derive(Debug)]
pub struct DllLayout {
    /// The segments defined in the DLL.
    pub segments: Vec<DllSegment>,
    /// The data layout for the DLL.
    pub data_layout: DataLayout,
}

/// A segment defined in the DLL.
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
#[display(
    fmt = "{}: vaddr={:#X}, size={:#X}",
    name,
    virtual_address,
    virtual_size
)]
pub struct DllSegment {
    /// The name of the segment.
    pub name: String,
    /// The virtual address that the segment is loaded to.
    ///
    /// This is the offset from the loaded DLL's base address.
    pub virtual_address: usize,
    /// The size that the segment has when loaded into memory.
    pub virtual_size: usize,
}

impl Display for DllLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "segments:")?;
        for segment in &self.segments {
            writeln!(f, "  {}", segment)?;
        }
        write!(f, "{}", self.data_layout)
    }
}

/// Construct a DllLayout from the DWARF debugging information in a DLL.
pub fn load_layout_from_dll(dll_path: impl AsRef<Path>) -> Result<DllLayout, LayoutError> {
    // Read object file
    let buffer = fs::read(&dll_path)?;
    let object = object::File::parse(&buffer[..])?;

    let mut segments = Vec::new();
    for segment in object.segments() {
        if let Some(name) = segment.name()? {
            segments.push(DllSegment {
                name: name.to_owned(),
                virtual_address: segment.address() as usize,
                virtual_size: segment.size() as usize,
            });
        }
    }

    // Load dwarf info
    let load_section = |id: SectionId| -> Result<Cow<'_, [u8]>, object::Error> {
        Ok(object
            .section_by_name(id.name())
            .map(|ref section| section.uncompressed_data())
            .transpose()?
            .unwrap_or(Cow::Borrowed(&[])))
    };
    let dwarf_cow = Dwarf::load(&load_section)?;
    let dwarf = dwarf_cow.borrow(|section| EndianSlice::new(&section, RunTimeEndian::default()));

    // Read layout from dwarf
    let data_layout = load_data_layout_from_dwarf(&dwarf)?;

    Ok(DllLayout {
        segments,
        data_layout,
    })
}

/// Build a DataLayout from the provided DWARF info.
fn load_data_layout_from_dwarf<R>(dwarf: &Dwarf<R>) -> Result<DataLayout, LayoutError>
where
    R: Reader,
    R::Offset: Display,
{
    let mut layout = DataLayout::new();

    // For each compilation unit within the dll
    let mut iter = dwarf.units();
    while let Some(header) = iter.next()? {
        let unit = dwarf.unit(header)?;
        let unit_name = match &unit.name {
            Some(name) => Some(name.to_string()?.as_ref().to_owned()),
            None => None,
        };

        // Extract layout information from each unit and merge into a single DataLayout
        let result: Result<(), LayoutError> = try {
            let mut unit_reader = UnitReader::new(dwarf, &unit);
            unit_reader.extract_definitions()?;
            // TODO: Use a more meaningful string than the TypeId
            unit_reader
                .update_layout(&mut layout)
                .map_err(|e| e.map(|id| format!("{}", id)))?;
        };

        result.map_err(|error| match unit_name {
            Some(name) => error.context(format!("in unit {}", name)),
            None => error,
        })?;
    }

    Ok(layout)
}

/// A placeholder id for a type reference within a compilation unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TypeId<O> {
    /// The offset to the type's dwarf entry.
    Offset(O),
    Void,
}

impl<O: Display> Display for TypeId<O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeId::Offset(offset) => write!(f, "{}", offset),
            TypeId::Void => write!(f, "void"),
        }
    }
}

/// State that is tracked when reading a single compilation unit.
struct UnitReader<'a, R: Reader> {
    dwarf: &'a Dwarf<R>,
    unit: &'a Unit<R>,
    /// The types that have been defined so far.
    ///
    /// DWARF defines types incrementally, e.g. writing typedef int Foo[] would
    /// create an entry for int, an entry for int[], and finally an entry for
    /// Foo. Thus not every defined type will correspond to a named type.
    /// These use PreDataTypes instead of DataTypes since they may reference
    /// types that have not been defined yet.
    pre_types: HashMap<TypeId<R::Offset>, PreDataType<TypeId<R::Offset>>>,
    /// Named type definitions.
    type_defns: HashMap<TypeName, ShallowDataType<TypeId<R::Offset>>>,
    /// Named variable definitions.
    global_defns: HashMap<String, ShallowDataType<TypeId<R::Offset>>>,
    /// Constant values.
    constants: HashMap<String, Constant>,
}

impl<'a, R> UnitReader<'a, R>
where
    R: Reader,
    R::Offset: Display,
{
    fn new(dwarf: &'a Dwarf<R>, unit: &'a Unit<R>) -> Self {
        Self {
            dwarf,
            unit,
            pre_types: HashMap::new(),
            type_defns: HashMap::new(),
            global_defns: HashMap::new(),
            constants: HashMap::new(),
        }
    }

    /// Extract type and variable definitions from the compilation unit dwarf info.
    fn extract_definitions(&mut self) -> Result<(), LayoutError> {
        // Define a void type for convenience since the dwarf info doesn't define one
        self.pre_types.insert(
            TypeId::Void,
            PreDataType {
                shallow_type: ShallowDataType::Void,
                size: PreDataTypeSize::Known(0),
            },
        );

        let mut tree: EntriesTree<'_, '_, R> = self.unit.entries_tree(None)?;
        let root = tree.root()?;

        self.expect_tag(root.entry(), gimli::DW_TAG_compile_unit)?;

        // Extract type and variable definitions from each dwarf entry
        let mut children = root.children();
        while let Some(node) = children.next()? {
            match node.entry().tag() {
                gimli::DW_TAG_base_type => self.read_base_type(node)?,
                gimli::DW_TAG_const_type => self.read_modified_type(node)?,
                gimli::DW_TAG_volatile_type => self.read_modified_type(node)?,
                gimli::DW_TAG_typedef => self.read_typedef(node)?,
                gimli::DW_TAG_pointer_type => self.read_pointer_type(node)?,
                gimli::DW_TAG_structure_type => self.read_struct_or_union_type(node)?,
                gimli::DW_TAG_union_type => self.read_struct_or_union_type(node)?,
                gimli::DW_TAG_enumeration_type => self.read_enumeration_type(node)?,
                gimli::DW_TAG_array_type => self.read_array_type(node)?,
                gimli::DW_TAG_subroutine_type => self.read_subroutine_type(node)?,
                gimli::DW_TAG_variable => self.read_variable(node)?,
                gimli::DW_TAG_subprogram => self.read_subprogram(node)?,
                _ => (),
            };
        }

        Ok(())
    }

    /// Update `layout` with the extracted type and variable definitions.
    ///
    /// Assumes that `extract_definitions` has been called.
    fn update_layout(
        &self,
        layout: &mut DataLayout,
    ) -> Result<(), BuildDataTypesError<TypeId<R::Offset>>> {
        // Resolve placeholder type ids and sizes/strides to build full data types
        let data_types = build_data_types(&self.pre_types)?;

        let get_type = |id: &TypeId<R::Offset>| data_types.get(id).cloned();
        let get_size = get_size_from_pre_types(&self.pre_types);

        // Resolve placeholder type ids in named type definitions
        for (type_name, shallow_type) in &self.type_defns {
            let data_type = shallow_type.resolve_direct(get_type, &get_size)?;
            layout.type_defns.insert(type_name.clone(), data_type);
        }

        // Resolve placeholder type ids in variables
        for (name, shallow_type) in &self.global_defns {
            let data_type = shallow_type.resolve_direct(get_type, &get_size)?;
            layout.globals.insert(name.clone(), data_type);
        }

        layout.constants.extend(
            self.constants
                .iter()
                .map(|(name, constant)| (name.clone(), constant.clone())),
        );

        Ok(())
    }

    fn read_base_type(&mut self, node: EntriesTreeNode<'_, '_, '_, R>) -> Result<(), LayoutError> {
        let entry = node.entry();
        let name = self.req_attr_string(entry, gimli::DW_AT_name)?;
        let shallow_type = match name.as_ref() {
            "char" => ShallowDataType::Int(IntType::S8),
            "long long unsigned int" => ShallowDataType::Int(IntType::U64),
            "long long int" => ShallowDataType::Int(IntType::S64),
            "short unsigned int" => ShallowDataType::Int(IntType::U16),
            "int" => ShallowDataType::Int(IntType::S32),
            "long int" => ShallowDataType::Int(IntType::S32),
            "unsigned int" => ShallowDataType::Int(IntType::U32),
            "long unsigned int" => ShallowDataType::Int(IntType::U32),
            "unsigned char" => ShallowDataType::Int(IntType::U8),
            "double" => ShallowDataType::Float(FloatType::F64),
            "float" => ShallowDataType::Float(FloatType::F32),
            "long double" => ShallowDataType::Void, // f128 is not currently supported
            "signed char" => ShallowDataType::Int(IntType::S8),
            "short int" => ShallowDataType::Int(IntType::S16),
            "_Bool" => ShallowDataType::Int(IntType::S32),
            _ => return Err(LayoutErrorCause::UnknownBaseTypeName { name }.into()),
        };
        self.pre_types.insert(
            TypeId::Offset(entry.offset().0),
            PreDataType {
                shallow_type,
                size: PreDataTypeSize::Known(self.req_attr_usize(entry, gimli::DW_AT_byte_size)?),
            },
        );
        Ok(())
    }

    fn read_modified_type(
        &mut self,
        node: EntriesTreeNode<'_, '_, '_, R>,
    ) -> Result<(), LayoutError> {
        // Ignore attributes and treat as a type alias
        let entry = node.entry();
        let target_type = self.req_attr_type_id(entry, gimli::DW_AT_type)?;
        self.pre_types.insert(
            TypeId::Offset(entry.offset().0),
            PreDataType {
                shallow_type: ShallowDataType::Alias(target_type),
                size: PreDataTypeSize::Defer(target_type),
            },
        );
        Ok(())
    }

    fn read_typedef(&mut self, node: EntriesTreeNode<'_, '_, '_, R>) -> Result<(), LayoutError> {
        let entry = node.entry();
        let type_name = TypeName {
            namespace: Namespace::Typedef,
            name: self.req_attr_string(entry, gimli::DW_AT_name)?,
        };
        let type_name_id = TypeId::Offset(entry.offset().0);

        let target_type_id = self.req_attr_type_id(entry, gimli::DW_AT_type)?;

        let data_type = PreDataType {
            shallow_type: ShallowDataType::Name(type_name.clone()),
            size: PreDataTypeSize::Defer(target_type_id),
        };
        self.pre_types.insert(type_name_id, data_type);

        self.type_defns
            .insert(type_name, ShallowDataType::Alias(target_type_id));
        Ok(())
    }

    fn read_pointer_type(
        &mut self,
        node: EntriesTreeNode<'_, '_, '_, R>,
    ) -> Result<(), LayoutError> {
        let entry = node.entry();
        self.pre_types.insert(
            TypeId::Offset(entry.offset().0),
            PreDataType {
                shallow_type: ShallowDataType::Pointer {
                    base: self.req_attr_type_id(entry, gimli::DW_AT_type)?,
                },
                size: PreDataTypeSize::Known(self.req_attr_usize(entry, gimli::DW_AT_byte_size)?),
            },
        );
        Ok(())
    }

    fn read_struct_or_union_type(
        &mut self,
        node: EntriesTreeNode<'_, '_, '_, R>,
    ) -> Result<(), LayoutError> {
        let entry = node.entry();

        let namespace = if entry.tag() == gimli::DW_TAG_structure_type {
            Namespace::Struct
        } else {
            Namespace::Union
        };
        let name = self.attr_string(entry, gimli::DW_AT_name)?;
        let type_id = TypeId::Offset(entry.offset().0);

        let size: PreDataTypeSize<_>;
        match self.attr_usize(entry, gimli::DW_AT_byte_size)? {
            None => {
                // This entry is a struct declaration, not definition
                let name = self.req_attr_string(entry, gimli::DW_AT_name)?;
                self.pre_types.insert(
                    type_id,
                    PreDataType {
                        shallow_type: ShallowDataType::Name(TypeName { namespace, name }),
                        size: PreDataTypeSize::Unknown,
                    },
                );
                return Ok(());
            }
            Some(s) => size = PreDataTypeSize::Known(s),
        }

        let mut field_info: Vec<(Option<String>, ShallowField<_>)> = Vec::new();

        // Read field entries
        let mut children = node.children();
        while let Some(field_node) = children.next()? {
            let field_entry = field_node.entry();
            self.expect_tag(field_entry, gimli::DW_TAG_member)?;
            field_info.push((
                self.attr_string(field_entry, gimli::DW_AT_name)?,
                ShallowField {
                    offset: if namespace == Namespace::Union {
                        0
                    } else {
                        self.req_attr_usize(field_entry, gimli::DW_AT_data_member_location)?
                    },
                    data_type: self.req_attr_type_id(field_entry, gimli::DW_AT_type)?,
                },
            ));
        }

        let mut used_field_names: HashSet<String> = field_info
            .iter()
            .filter_map(|field| field.0.clone())
            .collect();

        // Give anonymous fields unique names
        let mut fields: HashMap<String, ShallowField<TypeId<R::Offset>>> = HashMap::new();
        for (explicit_name, field) in field_info {
            let field_name =
                explicit_name.unwrap_or_else(|| unique_name(&used_field_names, ANON_FIELD_NAME));
            used_field_names.insert(field_name.clone());
            fields.insert(field_name, field);
        }

        let shallow_type = match namespace {
            Namespace::Struct => ShallowDataType::Struct { fields },
            Namespace::Union => ShallowDataType::Union { fields },
            _ => unimplemented!(),
        };

        match name {
            Some(name) => {
                // type id -> type name -> struct
                let type_name = TypeName { namespace, name };
                self.type_defns.insert(type_name.clone(), shallow_type);
                self.pre_types.insert(
                    type_id,
                    PreDataType {
                        shallow_type: ShallowDataType::Name(type_name),
                        size,
                    },
                );
            }
            None => {
                // type id -> struct
                self.pre_types
                    .insert(type_id, PreDataType { shallow_type, size });
            }
        };
        Ok(())
    }

    fn read_enumeration_type(
        &mut self,
        node: EntriesTreeNode<'_, '_, '_, R>,
    ) -> Result<(), LayoutError> {
        let entry = node.entry();
        let name = self.attr_string(entry, gimli::DW_AT_name)?;

        self.pre_types.insert(
            TypeId::Offset(entry.offset().0),
            PreDataType {
                shallow_type: ShallowDataType::Alias(
                    self.req_attr_type_id(entry, gimli::DW_AT_type)?,
                ),
                size: PreDataTypeSize::Known(self.req_attr_usize(entry, gimli::DW_AT_byte_size)?),
            },
        );

        // Read constant values
        let mut children = node.children();
        while let Some(variant_node) = children.next()? {
            let variant_entry = variant_node.entry();
            self.expect_tag(variant_entry, gimli::DW_TAG_enumerator)?;
            let variant_name = self.req_attr_string(variant_entry, gimli::DW_AT_name)?;

            let value = IntValue::from(self.req_attr_i64(variant_entry, gimli::DW_AT_const_value)?);
            let source = ConstantSource::Enum { name: name.clone() };

            self.constants
                .insert(variant_name, Constant { value, source });
        }

        Ok(())
    }

    fn read_array_type(&mut self, node: EntriesTreeNode<'_, '_, '_, R>) -> Result<(), LayoutError> {
        let entry = node.entry();

        let size = match self.attr_usize(entry, gimli::DW_AT_byte_size)? {
            Some(size) => PreDataTypeSize::Known(size),
            None => PreDataTypeSize::Unknown,
        };
        let type_id = TypeId::Offset(entry.offset().0);
        let base_type = self.req_attr_type_id(entry, gimli::DW_AT_type)?;
        let entry_label = self.entry_label(entry);

        // Read length from subrange child
        let mut children = node.children();
        let subrange_node = children
            .next()?
            .ok_or(LayoutErrorCause::MissingSubrangeNode { entry_label })?;
        let subrange_entry = subrange_node.entry();
        self.expect_tag(subrange_entry, gimli::DW_TAG_subrange_type)?;
        let length = self
            .attr_usize(subrange_entry, gimli::DW_AT_upper_bound)?
            .map(|n| n + 1);

        self.pre_types.insert(
            type_id,
            PreDataType {
                shallow_type: ShallowDataType::Array {
                    base: base_type,
                    length,
                },
                size,
            },
        );

        Ok(())
    }

    fn read_subroutine_type(
        &mut self,
        node: EntriesTreeNode<'_, '_, '_, R>,
    ) -> Result<(), LayoutError> {
        // TODO: Function types
        let entry = node.entry();
        self.pre_types.insert(
            TypeId::Offset(entry.offset().0),
            PreDataType {
                shallow_type: ShallowDataType::Void,
                size: PreDataTypeSize::Unknown,
            },
        );
        Ok(())
    }

    fn read_variable(&mut self, node: EntriesTreeNode<'_, '_, '_, R>) -> Result<(), LayoutError> {
        let entry = node.entry();
        if let Some(name) = self.attr_string(entry, gimli::DW_AT_name)? {
            self.global_defns.insert(
                name,
                ShallowDataType::Alias(self.req_attr_type_id(entry, gimli::DW_AT_type)?),
            );
        }
        Ok(())
    }

    fn read_subprogram(&mut self, node: EntriesTreeNode<'_, '_, '_, R>) -> Result<(), LayoutError> {
        // TODO: Functions
        let entry = node.entry();
        if let Some(name) = self.attr_string(entry, gimli::DW_AT_name)? {
            self.global_defns.insert(name, ShallowDataType::Void);
        }
        Ok(())
    }

    /// Read a string attribute from `entry`.
    ///
    /// Return None if the attribute is not present.
    /// Return an error if the attribute is present but is not a string.
    fn attr_string(
        &self,
        entry: &DebuggingInformationEntry<'_, '_, R>,
        attr_name: DwAt,
    ) -> Result<Option<String>, LayoutError> {
        Ok(match entry.attr_value(attr_name)? {
            Some(attr) => Some(
                self.dwarf
                    .attr_string(self.unit, attr)?
                    .to_string()?
                    .as_ref()
                    .to_owned(),
            ),
            None => None,
        })
    }

    /// Read an offset attribute from `entry`.
    ///
    /// Return None if the attribute is not present.
    /// Return an error if the attribute is present but is not an offset.
    fn attr_offset(
        &self,
        entry: &DebuggingInformationEntry<'_, '_, R>,
        attr_name: DwAt,
    ) -> Result<Option<R::Offset>, LayoutError> {
        Ok(entry.attr(attr_name)?.and_then(|attr| {
            if let AttributeValue::UnitRef(offset) = attr.value() {
                Some(offset.0)
            } else {
                None
            }
        }))
    }

    /// Read an unsigned int attribute from `entry`.
    ///
    /// Return None if the attribute is not present.
    /// Return an error if the attribute is present but is not an unsigned int.
    fn attr_usize(
        &self,
        entry: &DebuggingInformationEntry<'_, '_, R>,
        attr_name: DwAt,
    ) -> Result<Option<usize>, LayoutError> {
        Ok(entry
            .attr(attr_name)?
            .and_then(|attr| attr.udata_value().map(|udata| udata as usize)))
    }

    /// Read a signed int attribute from `entry`.
    //
    /// Return None if the attribute is not present.
    /// Return an error if the attribute is present but not a signed int.
    fn attr_i64(
        &self,
        entry: &DebuggingInformationEntry<'_, '_, R>,
        attr_name: DwAt,
    ) -> Result<Option<i64>, LayoutError> {
        Ok(entry.attr(attr_name)?.and_then(|attr| attr.sdata_value()))
    }

    /// Read a string attribute from `entry`.
    ///
    /// Return an error if the attribute is not present or is not a string.
    fn req_attr_string(
        &self,
        entry: &DebuggingInformationEntry<'_, '_, R>,
        attr_name: DwAt,
    ) -> Result<String, LayoutError> {
        self.attr_string(entry, attr_name)?
            .ok_or_else(|| self.missing_attribute(entry, attr_name))
    }

    /// Read an unsigned int attribute from `entry`.
    ///
    /// Return an error if the attribute is not present or is not an unsigned int.
    fn req_attr_usize(
        &self,
        entry: &DebuggingInformationEntry<'_, '_, R>,
        attr_name: DwAt,
    ) -> Result<usize, LayoutError> {
        self.attr_usize(entry, attr_name)?
            .ok_or_else(|| self.missing_attribute(entry, attr_name))
    }

    /// Read a signed int attribute from `entry`.
    ///
    /// Return an error if the attribute is not present or is not an signed int.
    fn req_attr_i64(
        &self,
        entry: &DebuggingInformationEntry<'_, '_, R>,
        attr_name: DwAt,
    ) -> Result<i64, LayoutError> {
        self.attr_i64(entry, attr_name)?
            .ok_or_else(|| self.missing_attribute(entry, attr_name))
    }

    /// Read a type id attribute from `entry`.
    ///
    /// Return TypeId::Void if the attribute is not present.
    /// Return an error if the attribute is not an offset.
    fn req_attr_type_id(
        &self,
        entry: &DebuggingInformationEntry<'_, '_, R>,
        attr_name: DwAt,
    ) -> Result<TypeId<R::Offset>, LayoutError> {
        Ok(match self.attr_offset(entry, attr_name)? {
            Some(offset) => TypeId::Offset(offset),
            None => TypeId::Void,
        })
    }

    /// Return a debug label for an entry.
    fn entry_label(&self, entry: &DebuggingInformationEntry<'_, '_, R>) -> String {
        match self.attr_string(entry, gimli::DW_AT_name) {
            Ok(Some(name)) => format!("{:?}: {}", entry.offset(), name),
            _ => format!("{:?}", entry.offset()),
        }
    }

    fn missing_attribute(
        &self,
        entry: &DebuggingInformationEntry<'_, '_, R>,
        attr_name: DwAt,
    ) -> LayoutError {
        LayoutErrorCause::MissingAttribute {
            entry_label: self.entry_label(entry),
            attribute: attr_name,
        }
        .into()
    }

    fn expect_tag(
        &self,
        entry: &DebuggingInformationEntry<'_, '_, R>,
        tag: gimli::DwTag,
    ) -> Result<(), LayoutError> {
        if entry.tag() == tag {
            Ok(())
        } else {
            Err(LayoutErrorCause::UnexpectedTag {
                entry_label: self.entry_label(entry),
                expected: tag,
                actual: entry.tag(),
            }
            .into())
        }
    }
}

/// The prefix used for naming anonymous fields.
const ANON_FIELD_NAME: &str = "__anon";

/// Return a name that isn't present in `used_names`.
fn unique_name(used_names: &HashSet<String>, base_name: &str) -> String {
    let fallbacks = (1..).map(|k| format!("{}_{}", base_name, k));
    iter::once(base_name.to_owned())
        .chain(fallbacks)
        .find(|name| !used_names.contains(name))
        .unwrap()
}
