use pyo3::prelude::*;
use wafel_viz::VizRenderData;

#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct VizScene {
    pub render_data: VizRenderData,
}
