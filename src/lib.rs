#![deny(missing_docs)]

//! Key Value Storage is a primitive, in-memory database with set, get, and remove methods accessed
//! via CLI.

use std::{collections::HashMap, fs::File, process};

///The main data structure that stores the values.
pub struct KvStore {
    map: HashMap<String, String>,
}

impl KvStore {
    /// Create an empty instance of the data structure.
    /// ```
    /// # use kvs::KvStore;
    /// let mut db = KvStore::new();
    /// ```
    pub fn new() -> KvStore {
        KvStore {
            map: HashMap::new(),
        }
    }

    /// The getter function.
    /// ```
    /// # use kvs::KvStore;
    /// # let mut db = KvStore::new();
    /// db.get("key".to_string());
    /// ```
    pub fn set(&mut self, key: String, value: String) {
        self.map.insert(key, value);
    }

    /// The setter function.
    /// ```
    /// # use kvs::KvStore;
    /// # let mut db = KvStore::new();
    /// db.set("key1".to_string(), "value1".to_string())
    /// ```
    pub fn get(&self, key: String) -> Option<String> {
        self.map.get(&key).cloned()
    }

    /// Remove values.
    /// ```
    /// # use kvs::KvStore;
    /// # let mut db = KvStore::new();
    /// db.remove("key1".to_string())
    /// ```
    pub fn remove(&mut self, key: String) {
        self.map.remove(&key).unwrap();
    }
}
