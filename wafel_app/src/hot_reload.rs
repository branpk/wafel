#[cfg(debug_assertions)]
pub use hot_wafel_app_ui::*;
#[cfg(not(debug_assertions))]
pub use wafel_app_ui::*;

#[cfg(debug_assertions)]
#[hot_lib_reloader::hot_module(dylib = "wafel_app_ui", lib_dir = "target/debug")]
mod hot_wafel_app_ui {
    pub use wafel_app_ui::{Env, Wafel};
    pub use wafel_viz::VizRenderData;

    hot_functions_from_file!("wafel_app_ui/src/lib.rs");
}
