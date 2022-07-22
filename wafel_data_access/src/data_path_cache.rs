use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{DataPathError, GlobalDataPath, LocalDataPath, MemoryLayout};

/// A cache for data path compilation.
#[derive(Debug, Default)]
pub struct DataPathCache {
    globals: Mutex<HashMap<String, Arc<GlobalDataPath>>>,
    locals: Mutex<HashMap<String, Arc<LocalDataPath>>>,
}

impl DataPathCache {
    pub fn global(
        &self,
        layout: &impl MemoryLayout,
        source: &str,
    ) -> Result<Arc<GlobalDataPath>, DataPathError> {
        let mut cache = self.globals.lock().unwrap();
        let value = cache.get(source);
        match value {
            Some(path) => Ok(Arc::clone(path)),
            None => {
                let path = Arc::new(GlobalDataPath::compile(layout, source)?);
                cache.insert(source.to_string(), path.clone());
                Ok(path)
            }
        }
    }

    pub fn local(
        &self,
        layout: &impl MemoryLayout,
        source: &str,
    ) -> Result<Arc<LocalDataPath>, DataPathError> {
        let mut cache = self.locals.lock().unwrap();
        let value = cache.get(source);
        match value {
            Some(path) => Ok(Arc::clone(path)),
            None => {
                let path = Arc::new(LocalDataPath::compile(layout, source)?);
                cache.insert(source.to_string(), path.clone());
                Ok(path)
            }
        }
    }
}
