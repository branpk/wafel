use super::SM64ErrorCause;
use crate::{error::Error, memory::AddressValue};
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
pub struct ObjectBehavior(pub AddressValue);

/// An abstract game variable, typically corresponding to a memory variable.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Variable {
    /// The internal name of the variable.
    pub name: Rc<String>,
    /// The frame that the variable is taken on.
    pub frame: Option<u32>,
    /// The object slot for object variables.
    pub object: Option<ObjectSlot>,
    /// The accepted object behavior for object variables.
    pub object_behavior: Option<ObjectBehavior>,
    /// The surface slot for surface variables.
    pub surface: Option<SurfaceSlot>,
}

impl Variable {
    /// Create a variable with the given name with no associated data.
    pub fn new(name: &str) -> Self {
        Self {
            name: Rc::new(name.to_owned()),
            frame: None,
            object: None,
            object_behavior: None,
            surface: None,
        }
    }

    /// Get the frame for the variable.
    ///
    /// # Panics
    ///
    /// Panics if the variable does not have a specified frame.
    pub fn frame_unwrap(&self) -> u32 {
        self.frame.expect("variable missing frame")
    }

    /// Get the frame for the variable.
    pub fn try_frame(&self) -> Result<u32, Error> {
        self.frame.ok_or_else(|| {
            SM64ErrorCause::MissingFrame {
                variable: self.to_string(),
            }
            .into()
        })
    }

    /// Get the object slot for the variable.
    pub fn try_object(&self) -> Result<ObjectSlot, Error> {
        self.object.ok_or_else(|| {
            SM64ErrorCause::MissingObject {
                variable: self.to_string(),
            }
            .into()
        })
    }

    /// Get the surface for the variable.
    pub fn try_surface(&self) -> Result<SurfaceSlot, Error> {
        self.surface.ok_or_else(|| {
            SM64ErrorCause::MissingSurface {
                variable: self.to_string(),
            }
            .into()
        })
    }

    /// Return a copy of the variable but associated with the given frame.
    pub fn with_frame(&self, frame: u32) -> Variable {
        let mut result = self.clone();
        result.frame = Some(frame);
        result
    }

    /// Return a copy of the variable but without an associated frame.
    pub fn without_frame(&self) -> Variable {
        let mut result = self.clone();
        result.frame = None;
        result
    }

    /// Return a copy of the variable but associated to the given object slot.
    pub fn with_object(&self, object: ObjectSlot) -> Self {
        let mut result = self.clone();
        result.object = Some(object);
        result
    }

    /// Return a copy of the variable but without an associated object slot.
    pub fn without_object(&self) -> Self {
        let mut result = self.clone();
        result.object = None;
        result
    }

    /// Return a copy of the variable but associated to the given object behavior.
    pub fn with_object_behavior(&self, behavior: ObjectBehavior) -> Self {
        let mut result = self.clone();
        result.object_behavior = Some(behavior);
        result
    }

    /// Return a copy of the variable but without an associated object behavior.
    pub fn without_object_behavior(&self) -> Self {
        let mut result = self.clone();
        result.object_behavior = None;
        result
    }

    /// Return a copy of the variable but associated to the given surface slot.
    pub fn with_surface(&self, surface: SurfaceSlot) -> Self {
        let mut result = self.clone();
        result.surface = Some(surface);
        result
    }

    /// Return a copy of the variable but without an associated surface slot.
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
