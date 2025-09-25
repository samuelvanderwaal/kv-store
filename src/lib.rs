//#![deny(missing_docs)]

//! Key Value Storage is a primitive, in-memory database with set, get, and remove methods accessed
//! via CLI.

use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{self, Seek, SeekFrom, Write},
    path::PathBuf,
};

use bson::Document;

use {
    serde::{Deserialize, Serialize},
    thiserror::Error,
};

/// Aliased result using the library's custom error.
pub type Result<T> = std::result::Result<T, KvError>;

/// Custom error for the library to represent the various types of failures.
#[derive(Debug, Error)]
pub enum KvError {
    #[error("Failed to open datastore at path")]
    Open(#[from] io::Error),
    #[error("Remove error")]
    Remove,
    #[error("bson error")]
    Bson(#[from] bson::error::Error),
    #[error("failed to insert into index")]
    IndexInsertion,
}

///The main data structure that stores the values.
pub struct KvStore {
    path: PathBuf,
    index: HashMap<String, u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Set { key: String, value: String },
    Get { key: String },
    Rm { key: String },
}

impl KvStore {
    /// Opens the KvStore at the given path.
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let p: PathBuf = path.into().join("kvstore.log");
        OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&p)?;

        let mut store = KvStore {
            path: p,
            index: HashMap::new(),
        };

        // Read the log to update the index.
        store.load()?;

        Ok(store)
    }

    /// The setter function.
    /// ```
    /// # use kvs::KvStore;
    /// # let mut db = KvStore::new();
    /// db.set("key1".to_string(), "value1".to_string())
    /// ```
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        let mut log = OpenOptions::new().append(true).open(&self.path)?;

        let start = log.stream_position()?;

        let command = Command::Set {
            key: key.clone(),
            value,
        };
        let doc = bson::serialize_to_document(&command)?;
        doc.to_writer(&log)?;
        log.flush()?;

        self.index.insert(key, start);

        Ok(())
    }

    /// The getter function.
    /// ```
    /// # use kvs::KvStore;
    /// # let mut db = KvStore::new();
    /// db.get("key".to_string());
    /// ```
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        let mut log = self.load()?;

        let pointer = self.index.get(&key);

        if let Some(p) = pointer {
            log.seek(SeekFrom::Start(*p))?;
            let doc = Document::from_reader(&log)?;
            let command: Command = bson::deserialize_from_document(doc)?;
            match command {
                Command::Set { key: _, value } => Ok(Some(value)),
                Command::Get { key: _ } => panic!("offset to invalid command!"),
                Command::Rm { key: _ } => panic!("offset to invalid command!"),
            }
        } else {
            Ok(None)
        }
    }

    /// Remove values.
    /// ```
    /// # use kvs::KvStore;
    /// # let mut db = KvStore::new();
    /// db.remove("key1".to_string())
    /// ```
    pub fn remove(&mut self, key: String) -> Result<()> {
        // Load index
        self.load()?;

        match self.index.remove(&key) {
            Some(_) => {
                let log = OpenOptions::new().append(true).open(&self.path)?;
                let command = Command::Rm { key };
                let doc = bson::serialize_to_document(&command)?;
                doc.to_writer(&log)?;
                Ok(())
            }
            None => Err(KvError::Remove),
        }
    }

    fn load(&mut self) -> Result<File> {
        let mut log = OpenOptions::new().read(true).open(&self.path)?;

        loop {
            let start = log.stream_position()?;
            let Ok(doc) = Document::from_reader(&mut log) else {
                break;
            };

            let command: Command = bson::deserialize_from_document(doc)?;

            match command {
                Command::Set { key, value: _ } => {
                    self.index.insert(key, start);
                }
                Command::Get { key: _ } => (),
                Command::Rm { key } => {
                    self.index.remove(&key);
                }
            }
        }

        Ok(log)
    }
}
