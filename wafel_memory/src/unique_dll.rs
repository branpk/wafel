use std::{
    fmt, fs,
    ops::Deref,
    path::{Path, PathBuf},
};

use dlopen::raw::Library;
use tempfile::TempPath;

use crate::DllLoadError;

/// A wrapper around [Library] that allows the same DLL to be opened multiple times.
///
/// Loading the same DLL file multiple times results in the same handle in memory.
/// This is a problem for [DllGameMemory](crate::DllGameMemory) since it requires
/// unique access to the DLL's global data.
///
/// To resolve this, [UniqueLibrary] first creates a copy of the DLL as a temp file,
/// and then opens that temp file instead.
pub(crate) struct UniqueLibrary {
    // `library` must be dropped before `temp_path`, since the temp file can't be deleted
    // while the DLL is still open.
    library: Library,
    original_path: PathBuf,
    temp_path: TempPath,
}

impl UniqueLibrary {
    pub(crate) fn open(dll_path: impl AsRef<Path>) -> Result<Self, DllLoadError> {
        let temp_file = tempfile::NamedTempFile::new()?;
        let dll_content = fs::read(&dll_path)?;
        fs::write(temp_file.path(), dll_content)?;

        // Close the temp file so that it can be re-opened as a DLL
        let temp_path = temp_file.into_temp_path();
        let library = Library::open(&temp_path)?;

        Ok(Self {
            library,
            original_path: dll_path.as_ref().to_owned(),
            temp_path,
        })
    }
}

impl Deref for UniqueLibrary {
    type Target = Library;

    fn deref(&self) -> &Self::Target {
        &self.library
    }
}

impl fmt::Debug for UniqueLibrary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UniqueDll")
            .field("original_path", &self.original_path)
            .field("temp_path", &self.temp_path)
            .finish_non_exhaustive()
    }
}
