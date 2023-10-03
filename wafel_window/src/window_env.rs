#[cfg(feature = "wafel_viz")]
use wafel_viz::VizRenderData;

use crate::Config;

/// Trait defining the interaction between a windowed application and the window.
pub trait WindowEnv {
    /// The config that was used when running the application.
    fn config(&self) -> &Config;

    /// A recent fps measurement.
    fn fps(&self) -> f32;

    /// A recent mspf measurement.
    fn mspf(&self) -> f32;

    /// The egui context.
    fn egui_ctx(&self) -> &egui::Context;

    /// Adds a [wafel_viz] visualization to the window.
    ///
    /// This method is only available when the `wafel_viz` feature is enabled.
    #[cfg(feature = "wafel_viz")]
    fn draw_viz(&self, render_data: VizRenderData);

    /// Return details of the most recent panic caught by the panic handler.
    ///
    /// This method also clears the panic details.
    fn take_recent_panic_details(&self) -> Option<String>;
}
