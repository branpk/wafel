#[cfg(debug_assertions)]
pub use hot_wafel_app_logic::*;
#[cfg(not(debug_assertions))]
pub use wafel_app_logic::*;

#[cfg(debug_assertions)]
#[hot_lib_reloader::hot_module(dylib = "wafel_app_logic", lib_dir = "target/debug")]
mod hot_wafel_app_logic {
    pub use wafel_app_logic::{Env, Wafel};
    pub use wafel_viz::VizRenderData;

    hot_functions_from_file!("wafel_app_logic/src/lib.rs");
}
