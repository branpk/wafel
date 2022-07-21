use std::{collections::HashMap, sync::Arc};

use wafel_data_type::Address;
use wafel_layout::{DataLayout, LayoutLookupError};
use wafel_memory::SymbolLookup;

use crate::{data_path_cache::DataPathCache, DataError, GlobalDataPath, LocalDataPath};

/// A trait for looking up the structured layout of data in memory.
pub trait MemoryLayout {
    /// Return the layout of data types and globals.
    fn data_layout(&self) -> &DataLayout;

    /// Look up a symbol in memory.
    fn symbol_address(&self, symbol: &str) -> Result<Address, DataError>;

    /// Return the name of the global variable at the given address.
    ///
    /// Returns None if no global variable is at the address.
    fn address_to_symbol(&self, addr: Address) -> Result<String, DataError>;

    /// Compile a global data path, cached.
    fn global_path(&self, source: &str) -> Result<Arc<GlobalDataPath>, DataError>;

    /// Compile a local data path, cached.
    fn local_path(&self, source: &str) -> Result<Arc<LocalDataPath>, DataError>;
}

/// Basic implementation of [MemoryLayout].
#[derive(Debug)]
#[allow(missing_docs)]
pub struct MemoryLayoutImpl<S> {
    pub data_layout: Arc<DataLayout>,
    pub symbol_lookup: Arc<S>,
    pub data_path_cache: DataPathCache<S>,
    pub address_to_symbol: HashMap<Address, String>,
}

impl<S> MemoryLayoutImpl<S>
where
    S: SymbolLookup,
{
    /// Construct a new [MemoryLayoutImpl].
    pub fn new(data_layout: &Arc<DataLayout>, symbol_lookup: &Arc<S>) -> Self {
        let address_to_symbol = data_layout
            .globals
            .keys()
            .filter_map(|name| {
                symbol_address(data_layout, symbol_lookup, name).map(|addr| (addr, name.clone()))
            })
            .collect();

        Self {
            data_layout: Arc::clone(data_layout),
            symbol_lookup: Arc::clone(symbol_lookup),
            data_path_cache: DataPathCache::new(symbol_lookup, data_layout),
            address_to_symbol,
        }
    }
}

impl<S> MemoryLayout for MemoryLayoutImpl<S>
where
    S: SymbolLookup,
{
    fn data_layout(&self) -> &DataLayout {
        &self.data_layout
    }

    fn symbol_address(&self, symbol: &str) -> Result<Address, DataError> {
        symbol_address(&self.data_layout, &self.symbol_lookup, symbol)
            .ok_or_else(|| LayoutLookupError::UndefinedGlobal(symbol.to_string()).into())
    }

    fn address_to_symbol(&self, addr: Address) -> Result<String, DataError> {
        self.address_to_symbol
            .get(&addr)
            .cloned()
            .ok_or(DataError::NoSymbolAtAddress(addr))
    }

    fn global_path(&self, source: &str) -> Result<Arc<GlobalDataPath>, DataError> {
        self.data_path_cache.global(source).map_err(DataError::from)
    }

    fn local_path(&self, source: &str) -> Result<Arc<LocalDataPath>, DataError> {
        self.data_path_cache.local(source).map_err(DataError::from)
    }
}

fn symbol_address(
    data_layout: &DataLayout,
    symbol_lookup: &impl SymbolLookup,
    symbol: &str,
) -> Option<Address> {
    data_layout
        .global(symbol)
        .ok()
        .and_then(|global| global.address)
        .map(|address| Address(address as usize))
        .or_else(|| symbol_lookup.symbol_address(symbol))
}
