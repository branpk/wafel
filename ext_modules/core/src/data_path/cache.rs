use super::DataPath;
use crate::{error::Error, memory::Memory};
use std::{cell::RefCell, collections::HashMap};

/// A cache for data path compilation.
#[derive(Debug, Clone)]
pub struct DataPathCache<M: Memory> {
    paths: RefCell<HashMap<String, DataPath<M>>>, // TODO: RwLock or thread-local?
}

impl<M: Memory> DataPathCache<M> {
    /// Construct an empty cache.
    pub fn new() -> Self {
        Self {
            paths: RefCell::new(HashMap::new()),
        }
    }

    /// Look up or compile a data path.
    pub fn path(&self, memory: &M, source: &str) -> Result<DataPath<M>, Error> {
        let mut paths = self.paths.borrow_mut();
        Ok(match paths.get(source) {
            Some(path) => path.clone(),
            None => {
                let path = DataPath::compile(memory, source)?;
                paths.entry(source.to_owned()).or_insert(path).clone()
            }
        })
    }
}
