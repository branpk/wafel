use fast3d::interpret::F3DRenderData;
use ultraviolet::{Vec2, Vec3};

use crate::{Camera, Element};

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct VizScene {
    pub viewport: Viewport,
    pub camera: Camera,
    pub f3d_render_data: Option<F3DRenderData>,
    pub elements: Vec<Element>,
}

impl Default for VizScene {
    fn default() -> Self {
        Self {
            viewport: Viewport::FullWindow,
            camera: Camera::orthographic(2.0, 2.0, -1.0, 1.0),
            f3d_render_data: None,
            elements: Vec::new(),
        }
    }
}

impl VizScene {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the position and size of the viewport on the window, in logical
    /// pixels (physical / scale_factor).
    ///
    /// If this is not called, then by default the viewport will take up the
    /// entire window.
    pub fn set_viewport_logical(&mut self, rect: Rect2) {
        self.viewport = Viewport::RectLogical(rect);
    }

    /// Sets the camera, which determines the projection and view matrices.
    ///
    /// If this is not called, then by default the camera is orthographic with
    /// bounds `(-1, -1, -1)` to `(1, 1, 1)`, looking down the negative z axis.
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
    }

    pub fn set_f3d_render_data(&mut self, f3d_render_data: F3DRenderData) {
        self.f3d_render_data = Some(f3d_render_data);
    }

    pub fn add(&mut self, element: impl Into<Element>) {
        self.elements.push(element.into());
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Viewport {
    FullWindow,
    RectLogical(Rect2),
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect2 {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect2 {
    pub fn zero() -> Self {
        Self {
            min: Vec2::zero(),
            max: Vec2::zero(),
        }
    }

    pub fn from_min_and_max(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub fn from_min_and_size(min: Vec2, size: Vec2) -> Self {
        Self {
            min,
            max: min + size,
        }
    }

    pub fn point(pos: Vec2) -> Self {
        Self { min: pos, max: pos }
    }

    pub fn min_x(&self) -> f32 {
        self.min.x
    }

    pub fn min_y(&self) -> f32 {
        self.min.y
    }

    pub fn max_x(&self) -> f32 {
        self.max.x
    }

    pub fn max_y(&self) -> f32 {
        self.max.y
    }

    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    pub fn size_x(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn size_y(&self) -> f32 {
        self.max.y - self.min.y
    }

    pub fn center(&self) -> Vec2 {
        (self.min + self.max) / 2.0
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.size_x() / self.size_y()
    }

    pub fn contains(&self, point: Vec2) -> bool {
        self.min.x <= point.x
            && point.x <= self.max.x
            && self.min.y <= point.y
            && point.y <= self.max.y
    }

    pub fn has_positive_area(&self) -> bool {
        self.min.x < self.max.x && self.min.y < self.max.y
    }

    pub fn has_nonnegative_area(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y
    }

    pub fn translate(&self, amount: Vec2) -> Self {
        Self {
            min: self.min + amount,
            max: self.max + amount,
        }
    }

    pub fn scale(&self, amount: f32) -> Self {
        Self {
            min: self.min * amount,
            max: self.max * amount,
        }
    }

    pub fn clamp(&self, bounds: Rect2) -> Self {
        Self {
            min: Vec2::new(
                self.min.x.clamp(bounds.min.x, bounds.max.x),
                self.min.y.clamp(bounds.min.y, bounds.max.y),
            ),
            max: Vec2::new(
                self.max.x.clamp(bounds.min.x, bounds.max.x),
                self.max.y.clamp(bounds.min.y, bounds.max.y),
            ),
        }
    }

    pub fn enclose(&self, other: Rect2) -> Self {
        Self {
            min: Vec2::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y)),
            max: Vec2::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect3 {
    pub min: Vec3,
    pub max: Vec3,
}

impl Rect3 {
    pub fn zero() -> Self {
        Self {
            min: Vec3::zero(),
            max: Vec3::zero(),
        }
    }

    pub fn from_min_and_max(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_min_and_size(min: Vec3, size: Vec3) -> Self {
        Self {
            min,
            max: min + size,
        }
    }

    pub fn point(pos: Vec3) -> Self {
        Self { min: pos, max: pos }
    }

    pub fn min_x(&self) -> f32 {
        self.min.x
    }

    pub fn min_y(&self) -> f32 {
        self.min.y
    }

    pub fn max_x(&self) -> f32 {
        self.max.x
    }

    pub fn max_y(&self) -> f32 {
        self.max.y
    }

    pub fn min_z(&self) -> f32 {
        self.min.z
    }

    pub fn max_z(&self) -> f32 {
        self.max.z
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn size_x(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn size_y(&self) -> f32 {
        self.max.y - self.min.y
    }

    pub fn size_z(&self) -> f32 {
        self.max.z - self.min.z
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) / 2.0
    }

    pub fn contains(&self, point: Vec3) -> bool {
        self.min.x <= point.x
            && point.x <= self.max.x
            && self.min.y <= point.y
            && point.y <= self.max.y
            && self.min.z <= point.z
            && point.z <= self.max.z
    }

    pub fn has_positive_area(&self) -> bool {
        self.min.x < self.max.x && self.min.y < self.max.y && self.min.z < self.max.z
    }

    pub fn has_nonnegative_area(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z
    }

    pub fn translate(&self, amount: Vec3) -> Self {
        Self {
            min: self.min + amount,
            max: self.max + amount,
        }
    }

    pub fn scale(&self, amount: f32) -> Self {
        Self {
            min: self.min * amount,
            max: self.max * amount,
        }
    }

    pub fn clamp(&self, bounds: Rect3) -> Self {
        Self {
            min: Vec3::new(
                self.min.x.clamp(bounds.min.x, bounds.max.x),
                self.min.y.clamp(bounds.min.y, bounds.max.y),
                self.min.z.clamp(bounds.min.z, bounds.max.z),
            ),
            max: Vec3::new(
                self.max.x.clamp(bounds.min.x, bounds.max.x),
                self.max.y.clamp(bounds.min.y, bounds.max.y),
                self.max.z.clamp(bounds.min.z, bounds.max.z),
            ),
        }
    }

    pub fn enclose(&self, other: Rect3) -> Self {
        Self {
            min: Vec3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            max: Vec3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        }
    }
}
