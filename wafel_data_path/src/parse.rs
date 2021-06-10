use std::{iter::Peekable, str::Chars};

use wafel_data_type::{Namespace, TypeName};

use crate::DataPathCompileError::{self, ParseError};

pub(crate) struct PathAst {
    pub(crate) root: RootAst,
    pub(crate) edges: Vec<EdgeAst>,
}

pub(crate) enum RootAst {
    Global(String),
    Local(TypeName),
}

pub(crate) enum EdgeAst {
    Field(String),
    Subscript(usize),
    Nullable,
}

pub(crate) fn parse_data_path(source: &str) -> Result<PathAst, DataPathCompileError> {
    Parser::new(source).parse()
}

struct Parser<'s> {
    chars: Peekable<Chars<'s>>,
}

impl<'s> Parser<'s> {
    fn new(source: &'s str) -> Self {
        Parser {
            chars: source.chars().peekable(),
        }
    }

    fn parse(mut self) -> Result<PathAst, DataPathCompileError> {
        self.skip_whitespace();
        self.path()
    }

    fn path(&mut self) -> Result<PathAst, DataPathCompileError> {
        let root = self.root()?;
        let mut edges = Vec::new();
        while self.chars.peek().is_some() {
            edges.push(self.edge()?);
        }
        Ok(PathAst { root, edges })
    }

    fn root(&mut self) -> Result<RootAst, DataPathCompileError> {
        let word = self.name()?;
        let root = match word.as_str() {
            "struct" => RootAst::Local(TypeName {
                namespace: Namespace::Struct,
                name: self.name()?,
            }),
            "union" => RootAst::Local(TypeName {
                namespace: Namespace::Union,
                name: self.name()?,
            }),
            "typedef" => RootAst::Local(TypeName {
                namespace: Namespace::Typedef,
                name: self.name()?,
            }),
            _ => RootAst::Global(word),
        };
        Ok(root)
    }

    fn edge(&mut self) -> Result<EdgeAst, DataPathCompileError> {
        match self.chars.peek() {
            Some('.') | Some('-') => self.field(),
            Some('[') => self.subscript(),
            Some('?') => self.nullable(),
            _ => Err(self.expected("expected `.`, `->`, `[`, or `?`")),
        }
    }

    fn field(&mut self) -> Result<EdgeAst, DataPathCompileError> {
        match self.chars.peek() {
            Some('.') => {
                self.chars.next();
            }
            Some('-') => {
                self.chars.next();
                self.char('>')?;
            }
            _ => return Err(self.expected("expected `.` or `->`")),
        }
        self.skip_whitespace();
        let name = self.name()?;
        Ok(EdgeAst::Field(name))
    }

    fn subscript(&mut self) -> Result<EdgeAst, DataPathCompileError> {
        self.char('[')?;
        self.skip_whitespace();
        let index = self.usize()?;
        self.char(']')?;
        self.skip_whitespace();
        Ok(EdgeAst::Subscript(index))
    }

    fn nullable(&mut self) -> Result<EdgeAst, DataPathCompileError> {
        self.char('?')?;
        self.skip_whitespace();
        Ok(EdgeAst::Nullable)
    }

    fn name(&mut self) -> Result<String, DataPathCompileError> {
        let mut name = String::new();
        match self.chars.peek() {
            Some(&c) if c.is_ascii_alphabetic() || c == '_' => {
                name.push(c);
                self.chars.next();
            }
            _ => return Err(self.expected("a variable name")),
        }

        while let Some(c) = self
            .chars
            .peek()
            .filter(|&&c| c.is_ascii_alphanumeric() || c == '_')
        {
            name.push(*c);
            self.chars.next();
        }

        self.skip_whitespace();
        Ok(name)
    }

    fn usize(&mut self) -> Result<usize, DataPathCompileError> {
        let mut digits = String::new();
        while let Some(&c) = self.chars.peek().filter(|c| c.is_ascii_digit()) {
            digits.push(c);
            self.chars.next();
        }

        if digits.is_empty() {
            return Err(self.expected("an unsigned integer"));
        }

        let result = digits
            .parse()
            .map_err(|_| ParseError(format!("integer out of range: {}", digits)))?;

        self.skip_whitespace();
        Ok(result)
    }

    fn skip_whitespace(&mut self) {
        while self
            .chars
            .peek()
            .filter(|c| c.is_ascii_whitespace())
            .is_some()
        {
            self.chars.next();
        }
    }

    fn char(&mut self, c: char) -> Result<(), DataPathCompileError> {
        if self.chars.peek() == Some(&c) {
            self.chars.next();
            Ok(())
        } else {
            Err(self.expected(format!("`{}`", c)))
        }
    }

    fn expected(&mut self, expected: impl Into<String>) -> DataPathCompileError {
        match self.chars.peek() {
            Some(c) => ParseError(format!("expected {}, found `{}`", expected.into(), c)),
            None => ParseError(format!(
                "expected {}, reached end of string",
                expected.into()
            )),
        }
    }
}
