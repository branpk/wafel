use wafel_api::VizRenderData;

use crate::{root::RootErrorBoundary, Env};

/// State of the Wafel application.
#[derive(Debug)]
pub struct Wafel {
    root: RootErrorBoundary,
}

impl Default for Wafel {
    fn default() -> Self {
        Self {
            root: RootErrorBoundary::new(),
        }
    }
}

impl Wafel {
    /// Render the Wafel UI and respond to user input events.
    pub fn show(&mut self, env: &dyn Env, ctx: &egui::Context) -> Vec<VizRenderData> {
        self.root.show(env, ctx)
    }
}
