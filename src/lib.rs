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
    uncompacted_bytes: u64,
}

const COMPACTION_THRESHOLD: u64 = 1024 * 1024; // 1MB threshold

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Command {
    Set { key: String, value: String },
    Get { key: String },
    Rm { key: String },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LogRecord {
    order: u64,
    command: Command,
}

impl KvStore {
    /// Opens the KvStore at the given path.
    /// ```
    /// # use kvs::{KvStore, Result};
    /// # fn main() -> Result<()> {
    /// let mut db = KvStore::open("./")?;
    /// # Ok(())
    /// # }
    /// ```
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
            uncompacted_bytes: 0,
        };

        // Read the log to update the index.
        store.load()?;

        Ok(store)
    }

    /// The setter function.
    /// ```
    /// # use kvs::{KvStore, Result};
    /// # fn main() -> Result<()> {
    /// # let mut db = KvStore::open("./")?;
    /// db.set("key1".to_string(), "value1".to_string())?;
    /// # Ok(())
    /// # }
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
        let end = log.stream_position()?;
        log.flush()?;

        self.index.insert(key, start);
        self.uncompacted_bytes += end - start;

        // Check if compaction is needed
        if self.uncompacted_bytes > COMPACTION_THRESHOLD {
            self.compact()?;
        }

        Ok(())
    }

    /// The getter function.
    /// ```
    /// # use kvs::{KvStore, Result};
    /// # fn main() -> Result<()> {
    /// # let mut db = KvStore::open("./")?;
    /// db.get("key".to_string())?;
    /// # Ok(())
    /// # }
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
    /// # use kvs::{KvStore, Result};
    /// # fn main() -> Result<()> {
    /// # let mut db = KvStore::open("./")?;
    /// # db.set("key1".to_string(), "value1".to_string())?;
    /// db.remove("key1".to_string())?;
    /// # Ok(())
    /// # }
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

        // Get the current file size as initial uncompacted bytes
        self.uncompacted_bytes = log.metadata()?.len();

        Ok(log)
    }

    fn compact(&mut self) -> Result<()> {
        let mut log = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&self.path)?;

        // Loop over the file reading each command
        // Get --> do nothing
        // Set or Rm --> update in-memory hashmap with latest key/command pair
        // Write commands back to log
        // Include nonce value to maintain original command order

        let mut commands: HashMap<String, LogRecord> = HashMap::new();
        let mut nonce = 0;

        loop {
            let Ok(doc) = Document::from_reader(&mut log) else {
                break;
            };

            let command: Command = bson::deserialize_from_document(doc)?;

            match &command {
                Command::Get { key: _ } => continue,
                Command::Set { key, value: _ } => {
                    commands.insert(
                        key.clone(),
                        LogRecord {
                            order: nonce,
                            command,
                        },
                    );
                    nonce += 1;
                }
                Command::Rm { key } => {
                    commands.remove(key);
                }
            }
        }

        drop(log);

        let mut log = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .open(&self.path)?;

        let mut records: Vec<&LogRecord> = commands.values().collect();
        records.sort();

        self.index.clear();
        for record in records {
            let start = log.stream_position()?;
            let doc = bson::serialize_to_document(&record.command)?;
            doc.to_writer(&log)?;

            // Rebuild the index with new offsets
            if let Command::Set { key, value: _ } = &record.command {
                self.index.insert(key.clone(), start);
            }
        }

        log.flush()?;
        self.uncompacted_bytes = 0;

        Ok(())
    }
}
