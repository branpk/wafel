use std::{
    env,
    path::{Path, PathBuf},
};

use hot_lib_reloader::LibReloadObserver;
use winit::window::Icon;

/// Configuration for the window and application environment.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Config {
    root_dir: PathBuf,
    relative_log_file_path: PathBuf,

    title: String,
    maximized: bool,
    icon: Option<Icon>,
    always_on_top: bool,

    hot_reload_subscriber: Option<fn() -> LibReloadObserver>,
}

static_assertions::assert_impl_all!(Config: Send, Sync);

impl Default for Config {
    fn default() -> Self {
        let root_dir = if cfg!(debug_assertions) {
            env::current_dir().expect("failed to locate current working directory")
        } else {
            let mut path = env::current_exe().expect("failed to locate executable");
            path.pop();
            path
        };

        Self {
            root_dir,
            relative_log_file_path: "log.txt".into(),

            title: String::new(),
            maximized: false,
            icon: None,
            always_on_top: cfg!(debug_assertions),

            hot_reload_subscriber: None,
        }
    }
}

impl Config {
    /// Returns the default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the directory that configuration and log files should be saved.
    ///
    /// By default:
    /// - In debug mode, this is the current working directory.
    /// - In release mode, this is the directory containing the executable.
    pub fn root_dir(&self) -> &Path {
        self.root_dir.as_path()
    }

    /// Sets the directory that configuration and log files should be saved.
    ///
    /// By default:
    /// - In debug mode, this is the current working directory.
    /// - In release mode, this is the directory containing the executable.
    pub fn with_root_dir(mut self, root_dir: impl AsRef<Path>) -> Self {
        self.root_dir = root_dir.as_ref().to_path_buf();
        self
    }

    /// Gets the log file path relative to the root directory.
    pub fn relative_log_file_path(&self) -> &Path {
        self.relative_log_file_path.as_path()
    }

    /// Sets the log file path relative to the root directory.
    pub fn with_relative_log_file_path(mut self, path: impl AsRef<Path>) -> Self {
        self.relative_log_file_path = path.as_ref().to_path_buf();
        self
    }

    /// Returns the absolute log file path.
    pub fn log_file_path(&self) -> PathBuf {
        self.root_dir.join(&self.relative_log_file_path)
    }

    /// Returns the window title.
    pub fn title(&self) -> &str {
        self.title.as_str()
    }

    /// Sets the window title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Returns whether the window should start maximized.
    pub fn maximized(&self) -> bool {
        self.maximized
    }

    /// Sets whether the window should start maximized.
    pub fn with_maximized(mut self, maximized: bool) -> Self {
        self.maximized = maximized;
        self
    }

    /// Returns the window icon (Windows only).
    pub fn icon(&self) -> Option<&Icon> {
        self.icon.as_ref()
    }

    /// Sets the window icon (Windows only).
    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Sets the window icon from a .ico file (Windows only).
    ///
    /// This requires the `image` feature.
    #[cfg(feature = "image")]
    pub fn with_icon_from_ico(self, ico_bytes: &[u8]) -> Self {
        let image = image::load_from_memory_with_format(ico_bytes, image::ImageFormat::Ico)
            .unwrap()
            .to_rgba8();
        let width = image.width();
        let height = image.height();
        let icon = Icon::from_rgba(image.into_raw(), width, height).unwrap();
        self.with_icon(icon)
    }

    /// Returns whether the window should stay on top of other windows.
    ///
    /// The default is true in debug mode and false in release mode.
    pub fn always_on_top(&self) -> bool {
        self.always_on_top
    }

    /// Sets whether the window should stay on top of other windows.
    ///
    /// The default is true in debug mode and false in release mode.
    pub fn with_always_on_top(mut self, always_on_top: bool) -> Self {
        self.always_on_top = always_on_top;
        self
    }

    /// Returns the hot reload subscriber if set.
    pub fn hot_reload_subscriber(&self) -> Option<fn() -> LibReloadObserver> {
        self.hot_reload_subscriber
    }

    /// If using hot reloading, this function should be called using
    /// `hot_module::subscribe` so that the window can block reloads when
    /// necessary to avoid crashes.
    pub fn with_hot_reload_observer(mut self, subscriber: fn() -> LibReloadObserver) -> Self {
        self.hot_reload_subscriber = Some(subscriber);
        self
    }
}
