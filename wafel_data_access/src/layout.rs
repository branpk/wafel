use std::sync::Arc;

use wafel_layout::DataLayout;
use wafel_memory::SymbolLookup;

use crate::{DataError, GlobalDataPath, LocalDataPath};

pub trait MemoryLayout {
    type SymbolLookup: SymbolLookup;

    fn data_layout(&self) -> &DataLayout;
    fn symbol_lookup(&self) -> &Self::SymbolLookup;
    fn global_path(&self, source: &str) -> Result<Arc<GlobalDataPath>, DataError>;
    fn local_path(&self, source: &str) -> Result<Arc<LocalDataPath>, DataError>;
}

#[derive(Debug)]
pub struct MemoryLayoutImpl<S> {
    data_layout: Arc<DataLayout>,
    symbol_lookup: Arc<S>,
}

impl<S> MemoryLayoutImpl<S> {
    pub fn new(data_layout: Arc<DataLayout>, symbol_lookup: Arc<S>) -> Self {
        Self {
            data_layout,
            symbol_lookup,
        }
    }
}

impl<S> MemoryLayout for MemoryLayoutImpl<S>
where
    S: SymbolLookup,
{
    type SymbolLookup = S;

    fn data_layout(&self) -> &DataLayout {
        &self.data_layout
    }

    fn symbol_lookup(&self) -> &Self::SymbolLookup {
        &self.symbol_lookup
    }

    fn global_path(&self, source: &str) -> Result<Arc<GlobalDataPath>, DataError> {
        todo!()
    }

    fn local_path(&self, source: &str) -> Result<Arc<LocalDataPath>, DataError> {
        todo!()
    }
}
