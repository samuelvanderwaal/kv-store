use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Seek, SeekFrom, Write},
    path::PathBuf,
};

use {
    bson::Document,
    serde::{Deserialize, Serialize},
};

use super::KvEngine;
use crate::{KvError, Result};

const COMPACTION_THRESHOLD: u64 = 1024 * 1024; // 1MB threshold

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "bincode", derive(bincode::{Encode, Decode}))]
pub enum KvCommand {
    Set { key: String, value: String },
    Get { key: String },
    Rm { key: String },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LogRecord {
    order: u64,
    command: KvCommand,
}

///The main data structure that stores the values.
pub struct KvStore {
    path: PathBuf,
    index: HashMap<String, u64>,
    uncompacted_bytes: u64,
    log_file: BufWriter<File>,
    read_file: BufReader<File>,
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
        let write_file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&p)?;

        let read_file = OpenOptions::new().read(true).open(&p)?;

        let mut store = KvStore {
            path: p,
            index: HashMap::new(),
            uncompacted_bytes: 0,
            log_file: BufWriter::new(write_file),
            read_file: BufReader::new(read_file),
        };

        // Read the log to update the index.
        store.load()?;

        Ok(store)
    }

    fn load(&mut self) -> Result<()> {
        // Flush any pending writes before reading
        self.log_file.flush()?;

        // Rewind to start of file
        self.read_file.seek(SeekFrom::Start(0))?;

        loop {
            let start = self.read_file.stream_position()?;
            let Ok(doc) = Document::from_reader(&mut self.read_file) else {
                break;
            };

            let command: KvCommand = bson::deserialize_from_document(doc)?;

            match command {
                KvCommand::Set { key, value: _ } => {
                    self.index.insert(key, start);
                }
                KvCommand::Get { key: _ } => (),
                KvCommand::Rm { key } => {
                    self.index.remove(&key);
                }
            }
        }

        // Get the current file size as initial uncompacted bytes
        self.uncompacted_bytes = self.read_file.get_ref().metadata()?.len();

        Ok(())
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

            let command: KvCommand = bson::deserialize_from_document(doc)?;

            match &command {
                KvCommand::Get { key: _ } => continue,
                KvCommand::Set { key, value: _ } => {
                    commands.insert(
                        key.clone(),
                        LogRecord {
                            order: nonce,
                            command,
                        },
                    );
                    nonce += 1;
                }
                KvCommand::Rm { key } => {
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
            if let KvCommand::Set { key, value: _ } = &record.command {
                self.index.insert(key.clone(), start);
            }
        }

        log.flush()?;
        self.uncompacted_bytes = 0;

        Ok(())
    }
}

impl KvEngine for KvStore {
    /// The setter function.
    /// ```
    /// # use kvs::{KvStore, Result};
    /// # fn main() -> Result<()> {
    /// # let mut db = KvStore::open("./")?;
    /// db.set("key1".to_string(), "value1".to_string())?;
    /// # Ok(())
    /// # }
    /// ```
    fn set(&mut self, key: String, value: String) -> Result<()> {
        let start = self.log_file.stream_position()?;

        let command = KvCommand::Set {
            key: key.clone(),
            value,
        };
        let doc = bson::serialize_to_document(&command)?;
        doc.to_writer(&mut self.log_file)?;
        let end = self.log_file.stream_position()?;

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
    fn get(&mut self, key: String) -> Result<Option<String>> {
        // Flush writes to ensure read_file sees all data
        self.log_file.flush()?;

        let pointer = self.index.get(&key);

        if let Some(p) = pointer {
            self.read_file.seek(SeekFrom::Start(*p))?;
            let doc = Document::from_reader(&mut self.read_file)?;
            let command: KvCommand = bson::deserialize_from_document(doc)?;
            match command {
                KvCommand::Set { key: _, value } => Ok(Some(value)),
                KvCommand::Get { key: _ } => panic!("offset to invalid command!"),
                KvCommand::Rm { key: _ } => panic!("offset to invalid command!"),
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
    fn rm(&mut self, key: String) -> Result<()> {
        // Load index
        self.load()?;

        match self.index.remove(&key) {
            Some(_) => {
                let command = KvCommand::Rm { key };
                let doc = bson::serialize_to_document(&command)?;
                doc.to_writer(&mut self.log_file)?;
                Ok(())
            }
            None => Err(KvError::Remove),
        }
    }
}
