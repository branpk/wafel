use std::{collections::HashMap, sync::Arc};

use deepsize::{known_deep_size, DeepSizeOf};
use lru::LruCache;
use wafel_data_access::GlobalDataPath;
use wafel_data_type::Value;
use wafel_memory::MemoryRead;

/// A cache for data path accesses, with the goal of minimizing frame requests.
///
/// Besides caching individual values, it also preloads certain paths as soon as a
/// frame is requested for the first time.
#[derive(Debug)]
pub(crate) struct DataCache {
    path_unintern: HashMap<usize, Arc<GlobalDataPath>>,
    hot_paths: LruCache<usize, ()>,
    cache: LruCache<u32, HashMap<usize, Value>>,
}

impl DataCache {
    pub(crate) fn new() -> Self {
        Self {
            path_unintern: HashMap::new(),
            hot_paths: LruCache::new(100),
            cache: LruCache::new(100),
        }
    }

    fn intern(&mut self, path: &Arc<GlobalDataPath>) -> usize {
        let id = Arc::as_ptr(path) as usize;
        self.path_unintern.insert(id, Arc::clone(path));
        id
    }

    fn unintern(&self, key: usize) -> &Arc<GlobalDataPath> {
        self.path_unintern.get(&key).unwrap()
    }

    pub(crate) fn get(&mut self, frame: u32, path: &Arc<GlobalDataPath>) -> Option<Value> {
        let path_key = self.intern(path);
        self.hot_paths.put(path_key, ());
        self.cache
            .get(&frame)
            .and_then(|frame_cache| frame_cache.get(&path_key))
            .cloned()
    }

    pub(crate) fn insert(&mut self, frame: u32, path: &Arc<GlobalDataPath>, value: Value) {
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

    pub(crate) fn preload_frame(&mut self, frame: u32, memory: &impl MemoryRead) {
        if self.cache.contains(&frame) {
            return;
        }
        let mut cache = HashMap::new();
        for (&path_key, ()) in &self.hot_paths {
            let path = self.unintern(path_key);
            // Ignore errors so that they can get caught when the path is directly requested
            if let Ok(value) = path.read(memory) {
                cache.insert(path_key, value);
            }
        }
        self.cache.put(frame, cache);
    }

    pub(crate) fn invalidate_frame(&mut self, invalidated_frame: u32) {
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

    pub(crate) fn byte_size(&self) -> usize {
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
