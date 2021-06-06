use super::State;
use crate::data_path::GlobalDataPath;
use deepsize::{known_deep_size, DeepSizeOf};
use lru::LruCache;
use std::collections::HashMap;
use wafel_data_type::Value;

/// A cache for data path accesses, with the goal of minimizing calls to `SlotManager#frame`.
///
/// Besides caching individual values, it also preloads certain paths as soon as a
/// frame is requested for the first time.
#[derive(Debug)]
pub struct DataCache {
    path_intern: HashMap<String, usize>,
    path_unintern: HashMap<usize, GlobalDataPath>,
    hot_paths: LruCache<usize, ()>,
    cache: LruCache<u32, HashMap<usize, Value>>,
}

impl DataCache {
    pub fn new() -> Self {
        Self {
            path_intern: HashMap::new(),
            path_unintern: HashMap::new(),
            hot_paths: LruCache::new(100),
            cache: LruCache::new(100),
        }
    }

    fn intern(&mut self, path: &GlobalDataPath) -> usize {
        match self.path_intern.get(path.source()) {
            Some(&key) => key,
            None => {
                let key = self.path_intern.len();
                self.path_intern.insert(path.source().to_owned(), key);
                self.path_unintern.insert(key, path.clone());
                key
            }
        }
    }

    fn unintern(&self, key: usize) -> &GlobalDataPath {
        self.path_unintern.get(&key).unwrap()
    }

    pub fn get(&mut self, frame: u32, path: &GlobalDataPath) -> Option<Value> {
        let path_key = self.intern(path);
        self.hot_paths.put(path_key, ());
        self.cache
            .get(&frame)
            .and_then(|cache| cache.get(&path_key))
            .cloned()
    }

    pub fn insert(&mut self, frame: u32, path: &GlobalDataPath, value: Value) {
        let path_key = self.intern(path);
        let cache = match self.cache.get_mut(&frame) {
            Some(cache) => cache,
            None => {
                self.cache.put(frame, HashMap::new());
                self.cache.peek_mut(&frame).unwrap()
            }
        };
        cache.insert(path_key, value);
    }

    pub fn preload_frame(&mut self, state: &impl State) {
        if self.cache.contains(&state.frame()) {
            return;
        }
        let mut cache = HashMap::new();
        for (&path_key, ()) in &self.hot_paths {
            let path = self.unintern(path_key);
            // Ignore errors so that they can get caught when the path is directly requested
            if let Ok(value) = state.path_read(path) {
                cache.insert(path_key, value);
            }
        }
        self.cache.put(state.frame(), cache);
    }

    pub fn invalidate_frame(&mut self, invalidated_frame: u32) {
        let invalidated_keys: Vec<u32> = self
            .cache
            .iter()
            .filter(|(&frame, _)| frame >= invalidated_frame)
            .map(|(&frame, _)| frame)
            .collect();

        for frame in invalidated_keys {
            self.cache.pop(&frame);
        }
    }

    pub fn byte_size(&self) -> usize {
        let mut entries: HashMap<u32, HashMap<usize, ValueWrapper>> = HashMap::new();
        for (frame, cache) in &self.cache {
            entries.insert(
                *frame,
                cache.iter().map(|(k, v)| (*k, v.clone().into())).collect(),
            );
        }
        entries.deep_size_of()
    }
}

#[derive(Debug, Clone)]
struct ValueWrapper(Value);

impl From<Value> for ValueWrapper {
    fn from(v: Value) -> Self {
        Self(v)
    }
}

known_deep_size!(0; ValueWrapper);
