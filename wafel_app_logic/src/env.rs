/// A trait defining the Wafel application's interaction with the file system
/// and windowing system.
///
/// The GUI implementation is in the wafel_app crate, but it can be overridden
/// to run Wafel in headless mode for example.
pub trait Env {
    /// Return the current version of Wafel.
    fn wafel_version(&self) -> &str;

    /// Return a list of running processes.
    fn processes(&self) -> Vec<ProcessInfo>;

    /// Return true if a process with the given pid is open.
    fn is_process_open(&self, pid: u32) -> bool;
}

/// The name and PID of a running process.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProcessInfo {
    /// PID of the process.
    pub pid: u32,
    /// Name of the process.
    pub name: String,
}
