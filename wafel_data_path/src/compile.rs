use std::sync::Arc;

use wafel_data_type::{Address, DataType, DataTypeRef, Field};
use wafel_layout::DataLayout;
use wafel_memory::SymbolLookup;

use crate::{
    parse::{parse_data_path, EdgeAst, RootAst},
    DataPath,
    DataPathCompileError::{self, *},
    DataPathEdge, DataPathError, DataPathImpl, GlobalDataPath, LocalDataPath,
};

pub(crate) fn data_path(
    layout: &Arc<DataLayout>,
    symbol_lookup: &impl SymbolLookup,
    source: &str,
) -> Result<DataPath, DataPathError> {
    data_path_impl(layout, symbol_lookup, source).map_err(|error| DataPathError::CompileError {
        source: source.to_string(),
        error,
    })
}

fn data_path_impl(
    layout: &Arc<DataLayout>,
    symbol_lookup: &impl SymbolLookup,
    source: &str,
) -> Result<DataPath, DataPathCompileError> {
    let ast = parse_data_path(source)?;

    let path = match ast.root {
        RootAst::Global(root_name) => {
            let root: Address =
                symbol_lookup
                    .symbol_address(&root_name)
                    .ok_or_else(|| UndefinedSymbol {
                        name: root_name.clone(),
                    })?;
            let root_type = layout.global(&root_name)?;
            let root_type = layout.concrete_type(root_type)?;

            let mut path = DataPathImpl {
                source: source.to_owned(),
                root,
                edges: Vec::new(),
                mask: None,
                concrete_type: root_type,
                layout: Arc::clone(layout),
            };

            for edge in ast.edges {
                path = follow_edge(layout, path, edge)?;
            }

            if let Some(mask) = ast.mask {
                if !path.concrete_type.is_int() {
                    return Err(MaskOnNonInt);
                }
                path.mask = Some(mask);
            }

            DataPath::Global(GlobalDataPath(path))
        }
        RootAst::Local(root_name) => {
            let root = layout.data_type(&root_name)?;
            let root = layout.concrete_type(root)?;

            let mut path = DataPathImpl {
                source: source.to_owned(),
                root: root.clone(),
                edges: Vec::new(),
                mask: None,
                concrete_type: root,
                layout: Arc::clone(layout),
            };

            for edge in ast.edges {
                path = follow_edge(layout, path, edge)?;
            }

            if let Some(mask) = ast.mask {
                if !path.concrete_type.is_int() {
                    return Err(MaskOnNonInt);
                }
                path.mask = Some(mask);
            }

            DataPath::Local(LocalDataPath(path))
        }
    };

    Ok(path)
}

fn follow_edge<T>(
    layout: &Arc<DataLayout>,
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
                    layout: Arc::clone(layout),
                };
            }
            follow_field_access(layout, path, field_name)
        }
        EdgeAst::Subscript(index) => {
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
                    layout: Arc::clone(layout),
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
    layout: &Arc<DataLayout>,
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
                    layout: Arc::clone(layout),
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
    layout: &Arc<DataLayout>,
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
                concrete_type: layout.concrete_type(&base)?,
                layout: Arc::clone(layout),
            })
        }
        _ => Err(NotAnArray),
    }
}
