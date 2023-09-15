use wafel_api::VizRenderData;

use crate::{error_boundary::ErrorBoundary, root::Root, Env};

/// State of the Wafel application.
#[derive(Debug)]
pub struct Wafel {
    error_boundary: ErrorBoundary,
    root: Root,
}

impl Default for Wafel {
    fn default() -> Self {
        Self {
            error_boundary: ErrorBoundary::new(),
            root: Root::new(),
        }
    }
}

impl Wafel {
    /// Render the Wafel UI and respond to user input events.
    pub fn show(&mut self, env: &dyn Env, ctx: &egui::Context) -> Vec<VizRenderData> {
        if self.error_boundary.has_error() {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.error_boundary.show_error(env, ui);
            });
            Vec::new()
        } else {
            self.error_boundary
                .catch_panic(env, || self.root.show(env, ctx))
                .unwrap_or_default()
        }
    }
}
