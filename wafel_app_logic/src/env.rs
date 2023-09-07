/// A trait defining the Wafel application's interaction with the external
/// environment.
pub trait Env {
    /// Return the current version of Wafel.
    fn wafel_version(&self) -> &str;
}
