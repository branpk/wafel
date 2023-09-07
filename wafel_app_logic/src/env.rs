/// A trait defining the Wafel application's interaction with the file system
/// and windowing system.
///
/// The GUI implementation is in the wafel_app crate, but it can be overridden
/// to run Wafel in headless mode for example.
pub trait Env {
    /// Return the current version of Wafel.
    fn wafel_version(&self) -> &str;
}
