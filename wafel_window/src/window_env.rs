use wafel_viz::VizScene;

use crate::AppConfig;

/// Trait defining the interaction between a windowed application and the window.
pub trait WindowEnv {
    /// The config that was used when running the application.
    fn config(&self) -> &AppConfig;

    /// A recent fps measurement.
    fn fps(&self) -> f32;

    /// A recent mspf measurement.
    fn mspf(&self) -> f32;

    /// The egui context.
    fn egui_ctx(&self) -> &egui::Context;

    /// Adds a [wafel_viz] visualization to the window.
    fn draw_viz(&self, scene: VizScene);

    /// Return details of the most recent panic caught by the panic handler.
    ///
    /// This method also clears the panic details.
    fn take_recent_panic_details(&self) -> Option<String>;
}
