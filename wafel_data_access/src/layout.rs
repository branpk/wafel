use std::{collections::HashMap, sync::Arc};

use wafel_data_type::{Address, DataTypeRef};
use wafel_layout::{DataLayout, LayoutLookupError};
use wafel_memory::SymbolLookup;

use crate::{
    data_path_cache::DataPathCache, readers::DataTypeReader, DataError, GlobalDataPath,
    LocalDataPath,
};

/// A trait for looking up the structured layout of data in memory.
pub trait MemoryLayout {
    /// Return the layout of data types and globals.
    fn data_layout(&self) -> &DataLayout;

    /// Return the size in bytes of a pointer (4 or 8).
    fn pointer_size(&self) -> usize;

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

    /// Returns a [DataReader](crate::DataReader) for reading a [Value](wafel_data_type::Value) of
    /// a given type.
    ///
    /// The reader can handle all data types except for:
    /// - Unsized arrays
    /// - Unions
    fn data_type_reader(&self, data_type: &DataTypeRef) -> Result<DataTypeReader, DataError> {
        let concrete_types = self.data_layout().concrete_types(data_type)?;
        Ok(DataTypeReader {
            data_type: data_type.clone(),
            concrete_types,
        })
    }
}

/// Basic implementation of [MemoryLayout].
#[derive(Debug)]
#[allow(missing_docs)]
pub struct MemoryLayoutImpl<S> {
    data_layout: Arc<DataLayout>,
    symbol_lookup: Arc<S>,
    pointer_size: usize,
    data_path_cache: DataPathCache,
    address_to_symbol: HashMap<Address, String>,
}

impl<S> MemoryLayoutImpl<S>
where
    S: SymbolLookup,
{
    /// Construct a new [MemoryLayoutImpl].
    pub fn new(data_layout: &Arc<DataLayout>, symbol_lookup: &Arc<S>, pointer_size: usize) -> Self {
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
            pointer_size,
            data_path_cache: DataPathCache::default(),
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

    fn pointer_size(&self) -> usize {
        self.pointer_size
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
        self.data_path_cache
            .global(self, source)
            .map_err(DataError::from)
    }

    fn local_path(&self, source: &str) -> Result<Arc<LocalDataPath>, DataError> {
        self.data_path_cache
            .local(self, source)
            .map_err(DataError::from)
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
