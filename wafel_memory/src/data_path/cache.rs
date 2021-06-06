use std::{collections::HashMap, sync::Mutex};

use wafel_layout::DataLayoutRef;

use crate::SymbolLookup;

use super::{DataPath, DataPathError};

/// A cache for data path compilation.
#[derive(Debug, Default)]
pub struct DataPathCache {
    paths: Mutex<HashMap<String, DataPath>>,
}

impl DataPathCache {
    /// Construct an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up or compile a data path.
    pub fn path(
        &self,
        layout: &DataLayoutRef,
        symbol_lookup: &impl SymbolLookup,
        source: &str,
    ) -> Result<DataPath, DataPathError> {
        let mut paths = self.paths.lock().unwrap();
        Ok(match paths.get(source) {
            Some(path) => path.clone(),
            None => {
                let path = DataPath::compile(layout, symbol_lookup, source)?;
                paths.entry(source.to_owned()).or_insert(path).clone()
            }
        })
    }
}
