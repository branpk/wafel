use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use wafel_data_path::{DataPathError, GlobalDataPath};
use wafel_layout::DataLayout;
use wafel_memory::DllGameMemory;

/// A cache for data path compilation.
#[derive(Debug)]
pub(crate) struct DataPathCache {
    memory: Arc<DllGameMemory>,
    layout: Arc<DataLayout>,
    paths: Mutex<HashMap<String, Arc<GlobalDataPath>>>,
}

impl DataPathCache {
    pub(crate) fn new(memory: &Arc<DllGameMemory>, layout: &Arc<DataLayout>) -> Self {
        Self {
            memory: Arc::clone(memory),
            layout: Arc::clone(layout),
            paths: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn get(&self, source: &str) -> Result<Arc<GlobalDataPath>, DataPathError> {
        let mut cache = self.paths.lock().unwrap();
        match cache.get(source) {
            Some(path) => Ok(Arc::clone(path)),
            None => {
                let path = Arc::new(GlobalDataPath::compile(&self.layout, &self.memory, source)?);
                cache.insert(source.to_string(), path.clone());
                Ok(path)
            }
        }
    }
}
