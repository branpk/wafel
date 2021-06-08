use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::{complete::space1, is_alphanumeric, is_digit},
    combinator::{all_consuming, map},
    error::{convert_error, ParseError, VerboseError},
    multi::many0,
    sequence::{preceded, separated_pair, terminated, tuple},
    Err, IResult,
};
use wafel_data_type::{Address, DataType, DataTypeRef, Field, Namespace, TypeName};
use wafel_layout::DataLayoutRef;
use wafel_memory::SymbolLookup;

use crate::{
    DataPath,
    DataPathCompileError::{self, *},
    DataPathEdge, DataPathError, DataPathImpl, GlobalDataPath, LocalDataPath,
};

pub fn data_path(
    layout: &DataLayoutRef,
    symbol_lookup: &impl SymbolLookup,
    source: &str,
) -> Result<DataPath, DataPathError> {
    data_path_impl(layout, symbol_lookup, source).map_err(|error| DataPathError::CompileError {
        source: source.to_string(),
        error,
    })
}

fn data_path_impl(
    layout: &DataLayoutRef,
    symbol_lookup: &impl SymbolLookup,
    source: &str,
) -> Result<DataPath, DataPathCompileError> {
    let (_, ast): (_, PathAst) = all_consuming(parse_path::<VerboseError<&str>>)(source)
        .map_err(|e| to_path_error(source, e))?;

    match ast.root {
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
                concrete_type: root_type,
                layout: DataLayoutRef::clone(layout),
            };

            for edge in ast.edges {
                path = follow_edge(layout, path, edge)?;
            }

            Ok(DataPath::Global(GlobalDataPath(path)))
        }
        RootAst::Local(root_name) => {
            let root = layout.data_type(&root_name)?;
            let root = layout.concrete_type(root)?;

            let mut path = DataPathImpl {
                source: source.to_owned(),
                root: root.clone(),
                edges: Vec::new(),
                concrete_type: root,
                layout: DataLayoutRef::clone(layout),
            };

            for edge in ast.edges {
                path = follow_edge(layout, path, edge)?;
            }

            Ok(DataPath::Local(LocalDataPath(path)))
        }
    }
}

fn follow_edge<T>(
    layout: &DataLayoutRef,
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
                    concrete_type: layout.concrete_type(base)?,
                    layout: DataLayoutRef::clone(layout),
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
                    concrete_type: DataTypeRef::new(DataType::Array {
                        base: base.clone(),
                        length: None,
                        stride,
                    }),
                    layout: DataLayoutRef::clone(layout),
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
    layout: &DataLayoutRef,
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
                    concrete_type: layout.concrete_type(data_type)?,
                    layout: DataLayoutRef::clone(layout),
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
    layout: &DataLayoutRef,
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
                concrete_type: layout.concrete_type(&base)?,
                layout: DataLayoutRef::clone(layout),
            })
        }
        _ => Err(NotAnArray),
    }
}

struct PathAst {
    root: RootAst,
    edges: Vec<EdgeAst>,
}

enum RootAst {
    Global(String),
    Local(TypeName),
}

enum EdgeAst {
    Field(String),
    Subscript(usize),
    Nullable,
}

fn to_path_error<'a>(source: &'a str, error: Err<VerboseError<&'a str>>) -> DataPathCompileError {
    DataPathCompileError::ParseError {
        message: match error {
            Err::Error(e) | Err::Failure(e) => convert_error(source, e),
            _ => "Incomplete".to_owned(),
        },
    }
}

fn parse_path<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, PathAst, E> {
    map(tuple((parse_root, many0(parse_edge))), |(root, edges)| {
        PathAst { root, edges }
    })(i)
}

fn parse_root<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, RootAst, E> {
    alt((
        map(parse_local_root, RootAst::Local),
        map(parse_global_root, RootAst::Global),
    ))(i)
}

fn parse_global_root<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, String, E> {
    map(parse_name, str::to_owned)(i)
}

fn parse_local_root<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, TypeName, E> {
    map(
        separated_pair(parse_namespace, space1, parse_name),
        |(namespace, name)| TypeName {
            namespace,
            name: name.to_owned(),
        },
    )(i)
}

fn parse_namespace<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Namespace, E> {
    alt((
        map(tag("struct"), |_| Namespace::Struct),
        map(tag("union"), |_| Namespace::Union),
        map(tag("typedef"), |_| Namespace::Typedef),
    ))(i)
}

fn parse_edge<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, EdgeAst, E> {
    alt((parse_field, parse_subscript, parse_nullable))(i)
}

fn parse_field<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, EdgeAst, E> {
    map(preceded(alt((tag("."), tag("->"))), parse_name), |name| {
        EdgeAst::Field(name.to_owned())
    })(i)
}

fn parse_subscript<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, EdgeAst, E> {
    map(
        preceded(tag("["), terminated(parse_int, tag("]"))),
        EdgeAst::Subscript,
    )(i)
}

fn parse_nullable<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, EdgeAst, E> {
    map(tag("?"), |_| EdgeAst::Nullable)(i)
}

fn parse_name<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    take_while1(|c| is_alphanumeric(c as u8) || c == '_')(i)
}

fn parse_int<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, usize, E> {
    map(take_while1(|c| is_digit(c as u8)), |s: &str| {
        s.parse::<usize>().unwrap()
    })(i)
}
