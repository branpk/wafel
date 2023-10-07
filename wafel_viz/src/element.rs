use ultraviolet::{Vec3, Vec4};

use crate::Rect3;

#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Point(PointElement),
    Line(LineElement),
    Triangle(TriangleElement),
}

impl Element {
    pub fn bounding_rect(&self) -> Rect3 {
        match self {
            Self::Point(point) => point.bounding_rect(),
            Self::Line(line) => line.bounding_rect(),
            Self::Triangle(triangle) => triangle.bounding_rect(),
        }
    }
}

impl From<PointElement> for Element {
    fn from(point: PointElement) -> Self {
        Self::Point(point)
    }
}

impl From<LineElement> for Element {
    fn from(line: LineElement) -> Self {
        Self::Line(line)
    }
}

impl From<TriangleElement> for Element {
    fn from(triangle: TriangleElement) -> Self {
        Self::Triangle(triangle)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct PointElement {
    pub pos: Vec3,
    pub size: f32,
    pub color: Vec4,
}

impl PointElement {
    pub fn new(pos: Vec3) -> Self {
        Self {
            pos,
            size: 1.0,
            color: [1.0, 1.0, 1.0, 1.0].into(),
        }
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = color;
        self
    }

    pub fn bounding_rect(&self) -> Rect3 {
        Rect3::point(self.pos)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct LineElement {
    pub vertices: [Vec3; 2],
    pub color: Vec4,
}

impl LineElement {
    pub fn new(vertices: [Vec3; 2]) -> Self {
        Self {
            vertices,
            color: [1.0, 1.0, 1.0, 1.0].into(),
        }
    }

    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = color;
        self
    }

    pub fn bounding_rect(&self) -> Rect3 {
        Rect3::point(self.vertices[0]).enclose(Rect3::point(self.vertices[1]))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriangleElement {
    pub vertices: [Vec3; 3],
    pub color: Vec4,
    pub surface_gradient: bool,
    pub transparency_hint: TransparencyHint,
}

impl TriangleElement {
    pub fn new(vertices: [Vec3; 3]) -> Self {
        Self {
            vertices,
            color: [1.0, 1.0, 1.0, 1.0].into(),
            surface_gradient: false,
            transparency_hint: TransparencyHint::None,
        }
    }

    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = color;
        self
    }

    pub fn with_surface_gradient(mut self, surface_gradient: bool) -> Self {
        self.surface_gradient = surface_gradient;
        self
    }

    /// Sets a hint for how to render a triangle with transparency, specifically
    /// to give wall hitboxes special behavior.
    ///
    /// Note that the renderer doesn't sort transparent objects.
    pub fn with_transparency_hint(mut self, transparency_hint: TransparencyHint) -> Self {
        self.transparency_hint = transparency_hint;
        self
    }

    pub fn bounding_rect(&self) -> Rect3 {
        Rect3::point(self.vertices[0])
            .enclose(Rect3::point(self.vertices[1]))
            .enclose(Rect3::point(self.vertices[2]))
    }
}

/// A hint for how to render a triangle with transparency, specifically to give
/// wall hitboxes special behavior.
///
/// Note that the renderer doesn't sort transparent objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TransparencyHint {
    /// Triangles with this transparency mode will be rendered after wall hitboxes,
    /// in the order they were added to the scene.
    ///
    /// This means that they will not be visible through wall hitboxes.
    #[default]
    None,
    /// Triangles with this transparency mode are only visible if they are the
    /// topmost WallHitbox triangle.
    WallHitbox,
}
