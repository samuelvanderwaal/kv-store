use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Seek, SeekFrom, Write},
    path::PathBuf,
};

use bincode::config::standard;
use serde::{Deserialize, Serialize};

use super::KvEngine;
use crate::{KvError, Result};

const COMPACTION_THRESHOLD: u64 = 1024 * 1024; // 1MB threshold

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    bincode::Encode,
    bincode::Decode,
)]
pub enum KvCommand {
    Set { key: String, value: String },
    Get { key: String },
    Rm { key: String },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, bincode::Encode, bincode::Decode)]
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
    write_pos: u64,
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
        let mut write_file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&p)?;

        // Seek to the end of the file for appending
        write_file.seek(SeekFrom::End(0))?;

        let read_file = OpenOptions::new().read(true).open(&p)?;

        let mut store = KvStore {
            path: p,
            index: HashMap::new(),
            uncompacted_bytes: 0,
            log_file: BufWriter::new(write_file),
            read_file: BufReader::new(read_file),
            write_pos: 0,
        };

        // Read the log to update the index.
        store.load()?;
        store.write_pos = store.read_file.get_ref().metadata()?.len();

        Ok(store)
    }

    fn load(&mut self) -> Result<()> {
        // Flush any pending writes before reading
        self.log_file.flush()?;

        // Rewind to start of file
        self.read_file.seek(SeekFrom::Start(0))?;

        loop {
            let start = self.read_file.stream_position()?;
            let Ok(command) = bincode::decode_from_reader(&mut self.read_file, standard()) else {
                break;
            };

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
        // Flush any pending writes before reading
        self.log_file.flush()?;

        let log = OpenOptions::new()
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

        // Create a BufReader once at the beginning
        let mut reader = BufReader::new(log);

        loop {
            let Ok(command) = bincode::decode_from_reader(&mut reader, standard()) else {
                break;
            };

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

        drop(reader);

        let mut log = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .open(&self.path)?;

        let mut records: Vec<&LogRecord> = commands.values().collect();
        records.sort();

        self.index.clear();
        let mut write_pos = 0;
        for record in records {
            let start = write_pos;

            let bytes = bincode::encode_to_vec(&record.command, standard())?;

            log.write_all(&bytes)?;
            write_pos += bytes.len() as u64;

            // Rebuild the index with new offsets
            if let KvCommand::Set { key, value: _ } = &record.command {
                self.index.insert(key.clone(), start);
            }
        }

        log.flush()?;
        self.uncompacted_bytes = 0;

        self.write_pos = write_pos;

        // Update file handles after compaction
        drop(log);
        let write_file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .append(true)
            .open(&self.path)?;
        let read_file = OpenOptions::new().read(true).open(&self.path)?;

        self.log_file = BufWriter::new(write_file);
        self.read_file = BufReader::new(read_file);

        Ok(())
    }
}

impl KvEngine for KvStore {
    /// The setter function.
    /// ```
    /// # use kvs::{KvStore, KvEngine, Result};
    /// # fn main() -> Result<()> {
    /// # let mut db = KvStore::open("./")?;
    /// db.set("key1".to_string(), "value1".to_string())?;
    /// # Ok(())
    /// # }
    /// ```
    fn set(&mut self, key: String, value: String) -> Result<()> {
        let start = self.write_pos;

        let command = KvCommand::Set {
            key: key.clone(),
            value,
        };
        let bytes = bincode::encode_to_vec(command, standard())?;

        self.log_file.write_all(&bytes)?;
        let bytes_written = bytes.len() as u64;

        self.write_pos += bytes_written;

        self.index.insert(key, start);
        self.uncompacted_bytes += bytes_written;

        // Check if compaction is needed
        if self.uncompacted_bytes > COMPACTION_THRESHOLD {
            self.compact()?;
        }

        Ok(())
    }

    /// The getter function.
    /// ```
    /// # use kvs::{KvStore, KvEngine, Result};
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
            let command: KvCommand = bincode::decode_from_reader(&mut self.read_file, standard())?;
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
    /// # use kvs::{KvStore, KvEngine, Result};
    /// # use tempfile::TempDir;
    /// # fn main() -> Result<()> {
    /// # let temp_dir = TempDir::new().expect("unable to create temp dir");
    /// # let mut db = KvStore::open(temp_dir.path())?;
    /// # db.set("key1".to_string(), "value1".to_string())?;
    /// db.rm("key1".to_string())?;
    /// # Ok(())
    /// # }
    /// ```
    fn rm(&mut self, key: String) -> Result<()> {
        match self.index.remove(&key) {
            Some(_) => {
                let command = KvCommand::Rm { key };
                let bytes = bincode::encode_to_vec(command, standard())?;

                self.log_file.write_all(&bytes)?;
                self.log_file.flush()?;

                self.write_pos += bytes.len() as u64;
                self.uncompacted_bytes += bytes.len() as u64;

                // Check if compaction is needed
                if self.uncompacted_bytes > COMPACTION_THRESHOLD {
                    self.compact()?;
                }

                Ok(())
            }
            None => Err(KvError::Remove),
        }
    }
}
