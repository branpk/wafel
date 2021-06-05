//! Non-recursive data types that can be resolved into data types.

#![allow(missing_docs)]

use super::{
    data_type::{DataType, DataTypeRef, Field, FloatType, IntType, TypeName},
    BuildDataTypesErrorCause,
};
use derive_more::Display;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
};
use topological_sort::TopologicalSort;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShallowField<Id> {
    pub offset: usize,
    pub data_type: Id,
}

#[derive(Debug, Display, Clone, PartialEq, Eq)]
pub enum ShallowDataType<Id> {
    #[display(fmt = "alias[{}]", _0)]
    Alias(Id),
    #[display(fmt = "void")]
    Void,
    #[display(fmt = "{}", _0)]
    Int(IntType),
    #[display(fmt = "{}", _0)]
    Float(FloatType),
    #[display(fmt = "ptr[{}]", base)]
    Pointer { base: Id },
    #[display(fmt = "array[{}, {:?}]", base, length)]
    Array { base: Id, length: Option<usize> },
    #[display(fmt = "struct[{:?}]", fields)]
    Struct {
        fields: HashMap<String, ShallowField<Id>>,
    },
    #[display(fmt = "struct[{:?}]", fields)]
    Union {
        fields: HashMap<String, ShallowField<Id>>,
    },
    #[display(fmt = "{}", _0)]
    Name(TypeName),
}

impl<Id: Debug> ShallowDataType<Id> {
    pub fn resolve_direct(
        &self,
        get_type_opt: impl Fn(&Id) -> Option<DataTypeRef>,
        get_size: impl Fn(&Id) -> Option<usize>,
        debug_name: &str,
    ) -> Result<DataTypeRef, BuildDataTypesErrorCause> {
        let get_type = |id| {
            get_type_opt(id).ok_or_else(|| BuildDataTypesErrorCause::UndefinedTypeId {
                id: format!("{:?}", id),
                context: format!("while resolving {}", debug_name),
            })
        };

        Ok(match self {
            ShallowDataType::Alias(id) => get_type(id)?,
            ShallowDataType::Void => DataTypeRef::new(DataType::Void),
            ShallowDataType::Int(int_type) => DataTypeRef::new(DataType::Int(*int_type)),
            ShallowDataType::Float(float_type) => DataTypeRef::new(DataType::Float(*float_type)),
            ShallowDataType::Pointer { base } => DataTypeRef::new(DataType::Pointer {
                base: get_type(base)?,
                stride: get_size(base),
            }),
            ShallowDataType::Array { base, length } => {
                let base_type = get_type(base)?;
                DataTypeRef::new(DataType::Array {
                    base: base_type.clone(),
                    length: *length,
                    stride: get_size(base).ok_or_else(|| {
                        BuildDataTypesErrorCause::UnsizedArrayElement {
                            array: debug_name.to_owned(),
                            element_type: base_type,
                        }
                    })?,
                })
            }
            ShallowDataType::Struct { fields } => DataTypeRef::new(DataType::Struct {
                fields: fields
                    .iter()
                    .map(|(name, field)| -> Result<_, _> {
                        Ok((
                            name.clone(),
                            Field {
                                offset: field.offset,
                                data_type: get_type(&field.data_type)?,
                            },
                        ))
                    })
                    .collect::<Result<HashMap<String, Field>, _>>()?,
            }),
            ShallowDataType::Union { fields } => DataTypeRef::new(DataType::Union {
                fields: fields
                    .iter()
                    .map(|(name, field)| -> Result<_, _> {
                        Ok((
                            name.clone(),
                            Field {
                                offset: field.offset,
                                data_type: get_type(&field.data_type)?,
                            },
                        ))
                    })
                    .collect::<Result<HashMap<String, Field>, _>>()?,
            }),
            ShallowDataType::Name(type_name) => DataTypeRef::new(DataType::Name(type_name.clone())),
        })
    }
}

#[derive(Debug, Display, Clone)]
#[display(
    fmt = "{}: {} (size = {:?})",
    "debug_name.clone().unwrap_or_else(|| \"?\".to_owned())",
    shallow_type,
    size
)]
pub struct PreDataType<Id> {
    pub debug_name: Option<String>,
    pub shallow_type: ShallowDataType<Id>,
    pub size: PreDataTypeSize<Id>,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreDataTypeSize<Id> {
    #[display(fmt = "Known({})", _0)]
    Known(usize),
    #[display(fmt = "Defer({})", _0)]
    Defer(Id),
    #[display(fmt = "Unknown")]
    Unknown,
}

impl<Id: Debug> PreDataType<Id> {
    fn debug_name(&self, id: &Id) -> String {
        match &self.debug_name {
            Some(debug_name) => format!("{:?}:{}", id, debug_name),
            None => format!("{:?}", id),
        }
    }

    pub fn dependencies(&self) -> Vec<&Id> {
        let mut dependencies = match &self.shallow_type {
            ShallowDataType::Alias(data_type) => vec![data_type],
            ShallowDataType::Void => Vec::new(),
            ShallowDataType::Int(_) => Vec::new(),
            ShallowDataType::Float(_) => Vec::new(),
            ShallowDataType::Pointer { base } => vec![base],
            ShallowDataType::Array { base, .. } => vec![base],
            ShallowDataType::Struct { fields } => fields.values().map(|f| &f.data_type).collect(),
            ShallowDataType::Union { fields } => fields.values().map(|f| &f.data_type).collect(),
            ShallowDataType::Name(_) => vec![],
        };
        if let PreDataTypeSize::Defer(id) = &self.size {
            dependencies.push(id);
        }
        dependencies
    }
}

pub fn get_size_from_pre_types<Id: Clone + Eq + Hash>(
    pre_types: &HashMap<Id, PreDataType<Id>>,
) -> impl Fn(&Id) -> Option<usize> + '_ {
    move |id: &Id| -> Option<usize> {
        let mut id = id.clone();
        loop {
            let pre_type = pre_types.get(&id)?;
            match pre_type.size.clone() {
                PreDataTypeSize::Defer(defer_id) => {
                    id = defer_id;
                }
                PreDataTypeSize::Known(size) => break Some(size),
                PreDataTypeSize::Unknown => break None,
            }
        }
    }
}

pub fn build_data_types<Id: Clone + Eq + Hash + Debug>(
    pre_types: &HashMap<Id, PreDataType<Id>>,
) -> Result<HashMap<Id, DataTypeRef>, BuildDataTypesErrorCause> {
    let mut sorter = TopologicalSort::<Id>::new();
    for (id, pre_type) in pre_types {
        sorter.insert(id.clone());
        for child_id in pre_type.dependencies() {
            sorter.add_dependency(child_id.clone(), id.clone());
        }
    }

    let get_size = get_size_from_pre_types(pre_types);

    let mut types: HashMap<Id, DataTypeRef> = HashMap::new();
    while let Some(id) = sorter.next() {
        let pre_data_type =
            pre_types
                .get(&id)
                .ok_or_else(|| BuildDataTypesErrorCause::UndefinedTypeId {
                    id: format!("{:?}", id),
                    context: "while resolving pre-types".to_owned(),
                })?;
        let data_type = pre_data_type.shallow_type.resolve_direct(
            |id| types.get(id).cloned(),
            &get_size,
            pre_data_type.debug_name(&id).as_ref(),
        )?;
        types.insert(id, data_type);
    }

    if sorter.is_empty() {
        Ok(types)
    } else {
        let mut remaining: HashSet<Id> = pre_types.keys().cloned().collect();
        for id in types.keys() {
            remaining.remove(id);
        }
        let debug_names: Vec<String> = remaining
            .iter()
            .map(|id| pre_types[id].debug_name(id))
            .collect();
        Err(BuildDataTypesErrorCause::CyclicDependency { nodes: debug_names })
    }
}
