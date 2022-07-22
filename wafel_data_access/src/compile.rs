use indexmap::IndexMap;
use wafel_data_type::{Address, DataType, DataTypeRef, Field, IntValue};
use wafel_layout::DataLayout;

use crate::{
    parse::{parse_data_path, EdgeAst, IntOrConstant, RootAst},
    DataPath,
    DataPathCompileError::{self, *},
    DataPathEdge, DataPathError, DataPathImpl, GlobalDataPath, LocalDataPath, MemoryLayout,
};

pub fn data_path(layout: &impl MemoryLayout, source: &str) -> Result<DataPath, DataPathError> {
    data_path_impl(layout, source).map_err(|error| DataPathError::CompileError {
        source: source.to_string(),
        error,
    })
}

fn data_path_impl(
    layout: &impl MemoryLayout,
    source: &str,
) -> Result<DataPath, DataPathCompileError> {
    let ast = parse_data_path(source)?;

    let data_layout = layout.data_layout();

    let path = match ast.root {
        RootAst::Global(root_name) => {
            let root: Address = layout
                .symbol_address(&root_name)
                .map_err(|_| UndefinedSymbol {
                    name: root_name.clone(),
                })?;

            let root_type = &data_layout.global(&root_name)?.data_type;
            let root_type = data_layout.concrete_type(root_type)?;

            let mut path = DataPathImpl {
                source: source.to_owned(),
                root,
                edges: Vec::new(),
                mask: None,
                concrete_type: root_type,
                concrete_types: IndexMap::new(),
            };

            for edge in ast.edges {
                path = follow_edge(data_layout, path, edge)?;
            }

            if let Some(mask) = ast.mask {
                if !path.concrete_type.is_int() {
                    return Err(MaskOnNonInt);
                }
                path.mask = Some(int_or_constant(data_layout, mask)?);
            }

            path.concrete_types = data_layout.concrete_types(&path.concrete_type)?;

            DataPath::Global(GlobalDataPath(path))
        }
        RootAst::Local(root_name) => {
            let root = data_layout.data_type(&root_name)?;
            let root = data_layout.concrete_type(root)?;

            let mut path = DataPathImpl {
                source: source.to_owned(),
                root: root.clone(),
                edges: Vec::new(),
                mask: None,
                concrete_type: root,
                concrete_types: IndexMap::new(),
            };

            for edge in ast.edges {
                path = follow_edge(data_layout, path, edge)?;
            }

            if let Some(mask) = ast.mask {
                if !path.concrete_type.is_int() {
                    return Err(MaskOnNonInt);
                }
                path.mask = Some(int_or_constant(data_layout, mask)?);
            }

            path.concrete_types = data_layout.concrete_types(&path.concrete_type)?;

            DataPath::Local(LocalDataPath(path))
        }
    };

    Ok(path)
}

fn int_or_constant(
    layout: &DataLayout,
    value: IntOrConstant,
) -> Result<IntValue, DataPathCompileError> {
    match value {
        IntOrConstant::Int(int) => Ok(int as IntValue),
        IntOrConstant::Constant(name) => {
            let constant = layout.constant(&name)?;
            Ok(constant.value)
        }
    }
}

fn follow_edge<T>(
    layout: &DataLayout,
    mut path: DataPathImpl<T>,
    edge: EdgeAst,
) -> Result<DataPathImpl<T>, DataPathCompileError> {
    match edge {
        EdgeAst::Field(field_name) => {
            if let DataType::Pointer { base, .. } = path.concrete_type.as_ref() {
                let mut edges = path.edges;
                edges.push(DataPathEdge::Deref);
                path = DataPathImpl {
                    source: path.source,
                    root: path.root,
                    edges,
                    mask: None,
                    concrete_type: layout.concrete_type(base)?,
                    concrete_types: path.concrete_types,
                };
            }
            follow_field_access(layout, path, field_name)
        }
        EdgeAst::Subscript(index) => {
            let index = int_or_constant(layout, index)? as usize;
            if let DataType::Pointer { base, stride } = path.concrete_type.as_ref() {
                let stride = if index == 0 {
                    // If index = 0, then stride doesn't matter, so use 0 if it's unknown
                    stride.unwrap_or(0)
                } else {
                    // If index =/= 0, then stride is required
                    stride.ok_or(UnsizedBaseType)?
                };
                let mut edges = path.edges;
                edges.push(DataPathEdge::Deref);
                path = DataPathImpl {
                    source: path.source,
                    root: path.root,
                    edges,
                    mask: None,
                    concrete_type: DataTypeRef::new(DataType::Array {
                        base: base.clone(),
                        length: None,
                        stride,
                    }),
                    concrete_types: path.concrete_types,
                };
            }
            follow_subscript(layout, path, index)
        }
        EdgeAst::Nullable => {
            if !path.concrete_type.is_pointer() {
                return Err(NullableNotAPointer);
            }
            path.edges.push(DataPathEdge::Nullable);
            Ok(path)
        }
    }
}

fn follow_field_access<T>(
    layout: &DataLayout,
    path: DataPathImpl<T>,
    field_name: String,
) -> Result<DataPathImpl<T>, DataPathCompileError> {
    match path.concrete_type.as_ref() {
        DataType::Struct { fields } | DataType::Union { fields } => match fields.get(&field_name) {
            Some(Field { offset, data_type }) => {
                let mut edges = path.edges;
                edges.push(DataPathEdge::Offset(*offset));
                Ok(DataPathImpl {
                    source: path.source,
                    root: path.root,
                    edges,
                    mask: None,
                    concrete_type: layout.concrete_type(data_type)?,
                    concrete_types: IndexMap::new(),
                })
            }
            None => Err(UndefinedField {
                name: field_name.clone(),
            }),
        },
        _ => Err(NotAStruct { field_name }),
    }
}

fn follow_subscript<T>(
    layout: &DataLayout,
    path: DataPathImpl<T>,
    index: usize,
) -> Result<DataPathImpl<T>, DataPathCompileError> {
    match path.concrete_type.as_ref() {
        DataType::Array {
            base,
            length,
            stride,
        } => {
            if let Some(length) = length {
                if index >= *length {
                    return Err(IndexOutOfBounds {
                        index,
                        length: *length,
                    });
                }
            }
            let mut edges = path.edges;
            edges.push(DataPathEdge::Offset(index * stride));
            Ok(DataPathImpl {
                source: path.source,
                root: path.root,
                edges,
                mask: None,
                concrete_type: layout.concrete_type(base)?,
                concrete_types: IndexMap::new(),
            })
        }
        _ => Err(NotAnArray),
    }
}
