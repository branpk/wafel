use super::SM64ErrorCause;
use crate::error::Error;
use derive_more::Display;
use std::{
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
pub struct Variable {
    pub name: Rc<String>,
    pub frame: Option<u32>,
    pub object: Option<ObjectSlot>,
    pub object_behavior: Option<ObjectBehavior>,
    pub surface: Option<SurfaceSlot>,
}

impl Variable {
    pub fn new(name: &str) -> Self {
        Self {
            name: Rc::new(name.to_owned()),
            frame: None,
            object: None,
            object_behavior: None,
            surface: None,
        }
    }

    pub fn frame_unwrap(&self) -> u32 {
        self.frame.expect("variable missing frame")
    }

    pub fn try_frame(&self) -> Result<u32, Error> {
        self.frame.ok_or_else(|| {
            SM64ErrorCause::MissingFrame {
                variable: self.to_string(),
            }
            .into()
        })
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
        let mut result = self.clone();
        result.frame = Some(frame);
        result
    }

    pub fn without_frame(&self) -> Variable {
        let mut result = self.clone();
        result.frame = None;
        result
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

impl Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut args = Vec::new();

        if let Some(frame) = self.frame {
            args.push(format!("frame={}", frame));
        }
        if let Some(object) = self.object {
            args.push(format!("object={}", object));
        }
        if let Some(object_behavior) = &self.object_behavior {
            args.push(format!("object_behavior={}", object_behavior));
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
