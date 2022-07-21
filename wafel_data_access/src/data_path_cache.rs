use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use wafel_layout::DataLayout;
use wafel_memory::SymbolLookup;

use crate::{DataPathError, GlobalDataPath, LocalDataPath};

/// A cache for data path compilation.
#[derive(Debug)]
pub struct DataPathCache<S> {
    symbol_lookup: Arc<S>,
    layout: Arc<DataLayout>,
    globals: Mutex<HashMap<String, Arc<GlobalDataPath>>>,
    locals: Mutex<HashMap<String, Arc<LocalDataPath>>>,
}

impl<S: SymbolLookup> DataPathCache<S> {
    pub fn new(symbol_lookup: &Arc<S>, layout: &Arc<DataLayout>) -> Self {
        Self {
            symbol_lookup: Arc::clone(symbol_lookup),
            layout: Arc::clone(layout),
            globals: Mutex::new(HashMap::new()),
            locals: Mutex::new(HashMap::new()),
        }
    }

    pub fn global(&self, source: &str) -> Result<Arc<GlobalDataPath>, DataPathError> {
        let mut cache = self.globals.lock().unwrap();
        let value = cache.get(source);
        match value {
            Some(path) => Ok(Arc::clone(path)),
            None => {
                let path = Arc::new(GlobalDataPath::compile(
                    &self.layout,
                    &self.symbol_lookup,
                    source,
                )?);
                cache.insert(source.to_string(), path.clone());
                Ok(path)
            }
        }
    }

    pub fn local(&self, source: &str) -> Result<Arc<LocalDataPath>, DataPathError> {
        let mut cache = self.locals.lock().unwrap();
        let value = cache.get(source);
        match value {
            Some(path) => Ok(Arc::clone(path)),
            None => {
                let path = Arc::new(LocalDataPath::compile(
                    &self.layout,
                    &self.symbol_lookup,
                    source,
                )?);
                cache.insert(source.to_string(), path.clone());
                Ok(path)
            }
        }
    }
}
