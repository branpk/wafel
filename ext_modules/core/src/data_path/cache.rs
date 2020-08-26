use super::DataPath;
use crate::{error::Error, memory::Memory};
use std::{collections::HashMap, sync::Mutex};

/// A cache for data path compilation.
#[derive(Debug)]
pub struct DataPathCache {
    paths: Mutex<HashMap<String, DataPath>>,
}

impl DataPathCache {
    /// Construct an empty cache.
    pub fn new() -> Self {
        Self {
            paths: Mutex::new(HashMap::new()),
        }
    }

    /// Look up or compile a data path.
    pub fn path(&self, memory: &impl Memory, source: &str) -> Result<DataPath, Error> {
        let mut paths = self.paths.lock().unwrap();
        Ok(match paths.get(source) {
            Some(path) => path.clone(),
            None => {
                let path = DataPath::compile(memory, source)?;
                paths.entry(source.to_owned()).or_insert(path).clone()
            }
        })
    }
}
