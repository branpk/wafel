use fast3d::interpret::F3DRenderData;
use ultraviolet::Mat4;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct VizScene {
    pub viewport_top_left_logical: Option<[u32; 2]>,
    pub viewport_size_logical: Option<[u32; 2]>,
    pub proj_mtx: [[f32; 4]; 4],
    pub view_mtx: [[f32; 4]; 4],
    pub f3d_render_data: Option<F3DRenderData>,
    pub elements: Vec<Element>,
}

impl Default for VizScene {
    fn default() -> Self {
        Self {
            viewport_top_left_logical: None,
            viewport_size_logical: None,
            proj_mtx: Mat4::identity().into(),
            view_mtx: Mat4::identity().into(),
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
    pub fn set_viewport_logical(&mut self, top_left: [u32; 2], size: [u32; 2]) -> &mut Self {
        self.viewport_top_left_logical = Some(top_left);
        self.viewport_size_logical = Some(size);
        self
    }

    pub fn set_proj_mtx(&mut self, proj_mtx: [[f32; 4]; 4]) -> &mut Self {
        self.proj_mtx = proj_mtx;
        self
    }

    pub fn set_view_mtx(&mut self, view_mtx: [[f32; 4]; 4]) -> &mut Self {
        self.view_mtx = view_mtx;
        self
    }

    pub fn set_f3d_render_data(&mut self, f3d_render_data: F3DRenderData) -> &mut Self {
        self.f3d_render_data = Some(f3d_render_data);
        self
    }

    pub fn add(&mut self, element: impl Into<Element>) -> &mut Self {
        self.elements.push(element.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Point(Point),
    Line(Line),
    Triangle(Triangle),
}

impl From<Point> for Element {
    fn from(point: Point) -> Self {
        Self::Point(point)
    }
}

impl From<Line> for Element {
    fn from(line: Line) -> Self {
        Self::Line(line)
    }
}

impl From<Triangle> for Element {
    fn from(triangle: Triangle) -> Self {
        Self::Triangle(triangle)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Point {
    pub pos: [f32; 3],
    pub size: f32,
    pub color: [f32; 4],
}

impl Point {
    pub fn new(pos: [f32; 3]) -> Self {
        Self {
            pos,
            size: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Line {
    pub vertices: [[f32; 3]; 2],
    pub color: [f32; 4],
}

impl Line {
    pub fn new(vertices: [[f32; 3]; 2]) -> Self {
        Self {
            vertices,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    pub vertices: [[f32; 3]; 3],
    pub color: [f32; 4],
    pub surface_gradient: bool,
    pub transparency_hint: TransparencyHint,
}

impl Triangle {
    pub fn new(vertices: [[f32; 3]; 3]) -> Self {
        Self {
            vertices,
            color: [1.0, 1.0, 1.0, 1.0],
            surface_gradient: false,
            transparency_hint: TransparencyHint::None,
        }
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
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
