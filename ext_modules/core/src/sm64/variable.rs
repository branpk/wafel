use super::SM64ErrorCause;
use crate::error::Error;
use derive_more::Display;
use std::{
    convert::TryFrom,
    fmt::{self, Display},
    rc::Rc,
};

/// A wrapper for an object slot index.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectSlot(pub usize);

/// A wrapper for a surface slot index.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceSlot(pub usize);

/// An opaque wrapper for an object behavior pointer.
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
pub struct ObjectBehavior(Rc<String>);

impl ObjectBehavior {
    pub(super) fn new<A: Display>(address: A) -> Self {
        Self(Rc::new(address.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariableGen<F> {
    pub name: Rc<String>,
    pub frame: F,
    pub object: Option<ObjectSlot>,
    pub object_behavior: Option<ObjectBehavior>,
    pub surface: Option<SurfaceSlot>,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Absent;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GenericFrame {
    Frame(u32),
    Absent,
}

pub type Variable = VariableGen<u32>;
pub type VariableWithoutFrame = VariableGen<Absent>;
pub type GenericVariable = VariableGen<GenericFrame>;

impl<F: Clone + Display> VariableGen<F> {
    pub fn new(name: &str) -> VariableWithoutFrame {
        VariableGen {
            name: Rc::new(name.to_owned()),
            frame: Absent,
            object: None,
            object_behavior: None,
            surface: None,
        }
    }

    fn map_frame<G>(self, func: impl FnOnce(F) -> G) -> VariableGen<G> {
        VariableGen {
            name: self.name,
            frame: func(self.frame),
            object: self.object,
            object_behavior: self.object_behavior,
            surface: self.surface,
        }
    }

    pub fn try_object(&self) -> Result<ObjectSlot, Error> {
        self.object.ok_or_else(|| {
            SM64ErrorCause::MissingObject {
                variable: self.to_string(),
            }
            .into()
        })
    }

    pub fn try_surface(&self) -> Result<SurfaceSlot, Error> {
        self.surface.ok_or_else(|| {
            SM64ErrorCause::MissingSurface {
                variable: self.to_string(),
            }
            .into()
        })
    }

    pub fn with_frame(&self, frame: u32) -> Variable {
        self.clone().map_frame(|_| frame)
    }

    pub fn without_frame(&self) -> VariableWithoutFrame {
        self.clone().map_frame(|_| Absent)
    }

    pub fn with_object(&self, object: ObjectSlot) -> Self {
        let mut result = self.clone();
        result.object = Some(object);
        result
    }

    pub fn without_object(&self) -> Self {
        let mut result = self.clone();
        result.object = None;
        result
    }

    pub fn with_object_behavior(&self, behavior: ObjectBehavior) -> Self {
        let mut result = self.clone();
        result.object_behavior = Some(behavior);
        result
    }

    pub fn without_object_behavior(&self) -> Self {
        let mut result = self.clone();
        result.object_behavior = None;
        result
    }

    pub fn with_surface(&self, surface: SurfaceSlot) -> Self {
        let mut result = self.clone();
        result.surface = Some(surface);
        result
    }

    pub fn without_surface(&self) -> Self {
        let mut result = self.clone();
        result.surface = None;
        result
    }
}

impl<F: Display> Display for VariableGen<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut args = Vec::new();

        let frame_str = self.frame.to_string();
        if frame_str != "Absent" {
            args.push(format!("frame={}", frame_str));
        }

        if let Some(object) = self.object {
            args.push(format!("object={}", object));
        }

        if let Some(surface) = self.surface {
            args.push(format!("surface={}", surface));
        }

        if args.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}[{}]", self.name, args.join(", "))
        }
    }
}

impl GenericVariable {
    pub fn try_frame(&self) -> Result<u32, Error> {
        match self.frame {
            GenericFrame::Frame(frame) => Ok(frame),
            GenericFrame::Absent => Err(SM64ErrorCause::MissingSurface {
                variable: self.to_string(),
            }
            .into()),
        }
    }
}

impl From<Variable> for GenericVariable {
    fn from(value: Variable) -> Self {
        value.map_frame(GenericFrame::Frame)
    }
}

impl From<VariableWithoutFrame> for GenericVariable {
    fn from(value: VariableWithoutFrame) -> Self {
        value.map_frame(|_| GenericFrame::Absent)
    }
}

impl TryFrom<GenericVariable> for Variable {
    type Error = Error;

    fn try_from(value: GenericVariable) -> Result<Self, Self::Error> {
        let frame = value.try_frame()?;
        Ok(value.map_frame(|_| frame))
    }
}
