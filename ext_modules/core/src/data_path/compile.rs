use super::{
    DataPath, DataPathEdge, DataPathErrorCause, DataPathImpl, GlobalDataPath, LocalDataPath,
};
use crate::{
    error::Error,
    memory::{
        data_type::{DataType, DataTypeRef, Field, Namespace, TypeName},
        AddressValue, DataLayout, Memory,
    },
};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::{complete::space1, is_alphanumeric, is_digit},
    combinator::{all_consuming, map, map_res},
    error::{convert_error, ParseError, VerboseError},
    multi::many0,
    sequence::{preceded, separated_pair, terminated, tuple},
    Err, IResult,
};

pub fn data_path<M: Memory>(memory: &M, source: &str) -> Result<DataPath, Error> {
    let result: Result<_, Error> = try {
        let (_, ast): (_, PathAst) = all_consuming(parse_path::<VerboseError<&str>>)(source)
            .map_err(|e| to_path_error(source, e))?;

        let layout = memory.data_layout();

        match ast.root {
            RootAst::Global(root_name) => {
                let root: AddressValue = memory.symbol_address(&root_name)?.into();
                let root_type = layout.get_global(&root_name)?;
                let root_type = layout.concrete_type(root_type)?;

                let mut path = DataPathImpl {
                    source: source.to_owned(),
                    root,
                    edges: Vec::new(),
                    concrete_type: root_type,
                };

                for edge in ast.edges {
                    path = follow_edge(layout, path, edge)?;
                }

                DataPath::Global(GlobalDataPath(path))
            }

            RootAst::Local(root_name) => {
                let root = layout.get_type(&root_name)?;
                let root = layout.concrete_type(root)?;

                let mut path = DataPathImpl {
                    source: source.to_owned(),
                    root: root.clone(),
                    edges: Vec::new(),
                    concrete_type: root,
                };

                for edge in ast.edges {
                    path = follow_edge(layout, path, edge)?;
                }

                DataPath::Local(LocalDataPath(path))
            }
        }
    };
    result.map_err(|error| error.context(format!("while compiling path {}", source)))
}

fn follow_edge<T>(
    layout: &DataLayout,
    mut path: DataPathImpl<T>,
    edge: EdgeAst,
) -> Result<DataPathImpl<T>, Error> {
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
                    stride.ok_or(DataPathErrorCause::UnsizedBaseType)?
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
                };
            }
            follow_subscript(layout, path, index)
        }
    }
}

fn follow_field_access<T>(
    layout: &DataLayout,
    path: DataPathImpl<T>,
    field_name: String,
) -> Result<DataPathImpl<T>, Error> {
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
                })
            }
            None => Err(DataPathErrorCause::UndefinedField {
                name: field_name.clone(),
            })?,
        },
        _ => Err(DataPathErrorCause::NotAStruct { field_name })?,
    }
}

fn follow_subscript<T>(
    layout: &DataLayout,
    path: DataPathImpl<T>,
    index: usize,
) -> Result<DataPathImpl<T>, Error> {
    match path.concrete_type.as_ref() {
        DataType::Array {
            base,
            length,
            stride,
        } => {
            if let Some(length) = length {
                if index >= *length {
                    Err(DataPathErrorCause::IndexOutOfBounds {
                        index,
                        length: *length,
                    })?;
                }
            }
            let mut edges = path.edges;
            edges.push(DataPathEdge::Offset(index * stride));
            Ok(DataPathImpl {
                source: path.source,
                root: path.root,
                edges,
                concrete_type: layout.concrete_type(&base)?,
            })
        }
        _ => Err(DataPathErrorCause::NotAnArray)?,
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
}

fn to_path_error<'a>(source: &'a str, error: Err<VerboseError<&'a str>>) -> Error {
    DataPathErrorCause::ParseError {
        message: match error {
            Err::Error(e) | Err::Failure(e) => convert_error(source, e),
            _ => "Incomplete".to_owned(),
        },
    }
    .into()
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
    alt((parse_field, parse_subscript))(i)
}

fn parse_field<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, EdgeAst, E> {
    map(preceded(alt((tag("."), tag("->"))), parse_name), |name| {
        EdgeAst::Field(name.to_owned())
    })(i)
}

fn parse_subscript<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, EdgeAst, E> {
    map(
        preceded(tag("["), terminated(parse_int, tag("]"))),
        |index| EdgeAst::Subscript(index),
    )(i)
}

fn parse_name<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    take_while1(|c| is_alphanumeric(c as u8) || c == '_')(i)
}

fn parse_int<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, usize, E> {
    map_res(take_while1(|c| is_digit(c as u8)), |s: &str| {
        s.parse::<usize>()
    })(i)
}
