#[cfg(feature = "reload")]
pub use hot_wafel_app_ui::*;
#[cfg(not(feature = "reload"))]
pub use wafel_app_ui::*;

#[cfg(feature = "reload")]
#[hot_lib_reloader::hot_module(
    dylib = "wafel_app_ui",
    lib_dir = "target/debug",
    file_watch_debounce = 500
)]
mod hot_wafel_app_ui {
    pub use wafel_app_ui::{Env, Wafel};
    pub use wafel_viz::VizScene;

    hot_functions_from_file!("wafel_app_ui/src/lib.rs");

    #[lib_change_subscription]
    pub fn subscribe() -> hot_lib_reloader::LibReloadObserver {}
}
