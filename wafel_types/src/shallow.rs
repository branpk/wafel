//! Non-recursive data types that can be resolved into data types.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    error::Error,
    fmt,
    hash::Hash,
};

use crate::{DataType, DataTypeRef, Field, FloatType, IntType, TypeName};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShallowField<Id> {
    pub offset: usize,
    pub data_type: Id,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShallowDataType<Id> {
    Alias(Id),
    Void,
    Int(IntType),
    Float(FloatType),
    Pointer {
        base: Id,
    },
    Array {
        base: Id,
        length: Option<usize>,
    },
    Struct {
        fields: HashMap<String, ShallowField<Id>>,
    },
    Union {
        fields: HashMap<String, ShallowField<Id>>,
    },
    Name(TypeName),
}

impl<Id: Clone> ShallowDataType<Id> {
    pub fn resolve_direct(
        &self,
        get_type_opt: impl Fn(&Id) -> Option<DataTypeRef>,
        get_size: impl Fn(&Id) -> Option<usize>,
    ) -> Result<DataTypeRef, BuildDataTypesError<Id>> {
        let get_type =
            |id| get_type_opt(id).ok_or_else(|| BuildDataTypesError::UndefinedTypeId(id.clone()));

        let data_type = match self {
            ShallowDataType::Alias(id) => return get_type(id),
            ShallowDataType::Void => DataType::Void,
            ShallowDataType::Int(int_type) => DataType::Int(*int_type),
            ShallowDataType::Float(float_type) => DataType::Float(*float_type),
            ShallowDataType::Pointer { base } => DataType::Pointer {
                base: get_type(base)?,
                stride: get_size(base),
            },
            ShallowDataType::Array { base, length } => DataType::Array {
                base: get_type(base)?,
                length: *length,
                stride: get_size(base)
                    .ok_or_else(|| BuildDataTypesError::UnsizedArrayElement(base.clone()))?,
            },
            ShallowDataType::Struct { fields } => {
                let mut resolved_fields = HashMap::new();
                for (name, field) in fields {
                    resolved_fields.insert(
                        name.clone(),
                        Field {
                            offset: field.offset,
                            data_type: get_type(&field.data_type)?,
                        },
                    );
                }
                DataType::Struct {
                    fields: resolved_fields,
                }
            }
            ShallowDataType::Union { fields } => {
                let mut resolved_fields = HashMap::new();
                for (name, field) in fields {
                    resolved_fields.insert(
                        name.clone(),
                        Field {
                            offset: field.offset,
                            data_type: get_type(&field.data_type)?,
                        },
                    );
                }
                DataType::Union {
                    fields: resolved_fields,
                }
            }
            ShallowDataType::Name(type_name) => DataType::Name(type_name.clone()),
        };
        Ok(DataTypeRef::new(data_type))
    }
}

#[derive(Debug, Clone)]
pub struct PreDataType<Id> {
    pub shallow_type: ShallowDataType<Id>,
    pub size: PreDataTypeSize<Id>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreDataTypeSize<Id> {
    Known(usize),
    Defer(Id),
    Unknown,
}

impl<Id> PreDataType<Id> {
    pub fn dependencies(&self) -> Vec<&Id> {
        let mut dependencies = match &self.shallow_type {
            ShallowDataType::Alias(data_type) => vec![data_type],
            ShallowDataType::Void => vec![],
            ShallowDataType::Int(_) => vec![],
            ShallowDataType::Float(_) => vec![],
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

pub fn build_data_types<Id: Clone + Eq + Hash>(
    pre_types: &HashMap<Id, PreDataType<Id>>,
) -> Result<HashMap<Id, DataTypeRef>, BuildDataTypesError<Id>> {
    let dependencies: HashMap<&Id, Vec<&Id>> = pre_types
        .iter()
        .map(|(id, pre_type)| (id, pre_type.dependencies()))
        .collect();
    let sorted = topological_sort(&dependencies)
        .map_err(|cycle| BuildDataTypesError::CyclicDependency(cycle.clone()))?;

    let get_size = get_size_from_pre_types(pre_types);

    let mut types: HashMap<Id, DataTypeRef> = HashMap::new();
    for &id in &sorted {
        let pre_data_type = pre_types
            .get(&id)
            .ok_or_else(|| BuildDataTypesError::UndefinedTypeId(id.clone()))?;
        let data_type = pre_data_type
            .shallow_type
            .resolve_direct(|id| types.get(id).cloned(), &get_size)?;
        types.insert(id.clone(), data_type);
    }

    Ok(types)
}

#[derive(Debug, Clone)]
pub enum BuildDataTypesError<Id> {
    UndefinedTypeId(Id),
    CyclicDependency(Id),
    UnsizedArrayElement(Id),
}

impl<Id: fmt::Display> fmt::Display for BuildDataTypesError<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildDataTypesError::UndefinedTypeId(id) => write!(f, "undefined type id: {}", id),
            BuildDataTypesError::CyclicDependency(id) => {
                write!(f, "cyclic type definition starting at: {}", id)
            }
            BuildDataTypesError::UnsizedArrayElement(id) => {
                write!(f, "unsized array element type: {}", id)
            }
        }
    }
}

impl<Id: fmt::Debug + fmt::Display> Error for BuildDataTypesError<Id> {}

fn topological_sort<T: Eq + Hash + Copy>(dependencies: &HashMap<T, Vec<T>>) -> Result<Vec<T>, T> {
    let all_nodes: HashSet<T> = dependencies
        .keys()
        .chain(dependencies.values().flatten())
        .copied()
        .collect();

    let mut num_dependencies: HashMap<T, usize> = all_nodes
        .iter()
        .map(|&n| (n, dependencies.get(&n).map(Vec::len).unwrap_or_default()))
        .collect();

    let mut dependents: HashMap<T, Vec<T>> = HashMap::new();
    for (&id, deps) in dependencies {
        for &dep in deps {
            dependents.entry(dep).or_default().push(id);
        }
    }

    let mut queue: VecDeque<T> = all_nodes
        .iter()
        .copied()
        .filter(|&n| num_dependencies[&n] == 0)
        .collect();
    let mut seen: HashSet<T> = HashSet::new();
    let mut result: Vec<T> = Vec::new();

    while let Some(node) = queue.pop_front() {
        if seen.insert(node) {
            result.push(node);
            for succ in dependents.remove(&node).unwrap_or_default() {
                if !seen.contains(&succ) {
                    let num_deps = num_dependencies.get_mut(&succ).unwrap();
                    assert_ne!(*num_deps, 0);
                    *num_deps -= 1;
                    if *num_deps == 0 {
                        queue.push_back(succ);
                    }
                }
            }
        }
    }

    if seen.len() < all_nodes.len() {
        let cycle_node = *all_nodes.iter().find(|&n| !seen.contains(n)).unwrap();
        Err(cycle_node)
    } else {
        Ok(result)
    }
}
