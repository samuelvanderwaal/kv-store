use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use {
    bincode::config::standard,
    dashmap::DashMap,
    serde::{Deserialize, Serialize},
};

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
#[derive(Clone)]
pub struct KvStore {
    inner: Arc<KvStoreInner>,
}

pub struct KvStoreInner {
    path: PathBuf,
    index: DashMap<String, u64>,
    uncompacted_bytes: AtomicU64,
    read_file: Mutex<BufReader<File>>,
    write_state: Mutex<WriteState>,
}

struct WriteState {
    log_file: BufWriter<File>,
    write_pos: AtomicU64,
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

        let store = KvStore {
            inner: Arc::new(KvStoreInner {
                path: p,
                index: DashMap::new(),
                uncompacted_bytes: AtomicU64::new(0),
                write_state: Mutex::new(WriteState {
                    log_file: BufWriter::new(write_file),
                    write_pos: AtomicU64::new(0),
                }),
                read_file: Mutex::new(BufReader::new(read_file)),
            }),
        };

        // Read the log to update the index.
        store.load()?;
        {
            let f = store.inner.read_file.lock()?;
            let length = f.get_ref().metadata()?.len();
            store
                .inner
                .write_state
                .lock()?
                .write_pos
                .store(length, Ordering::SeqCst);
        }

        Ok(store)
    }

    fn load(&self) -> Result<()> {
        // Flush any pending writes before reading
        self.inner.write_state.lock()?.log_file.flush()?;

        // Rewind to start of file
        self.inner.read_file.lock()?.seek(SeekFrom::Start(0))?;

        let mut read_file = self.inner.read_file.lock()?;
        loop {
            let start = read_file.stream_position()?;
            let Ok(command) = bincode::decode_from_reader(&mut *read_file, standard()) else {
                break;
            };

            match command {
                KvCommand::Set { key, value: _ } => {
                    self.inner.index.insert(key, start);
                }
                KvCommand::Get { key: _ } => (),
                KvCommand::Rm { key } => {
                    self.inner.index.remove(&key);
                }
            }
        }

        // Get the current file size as initial uncompacted bytes
        self.inner
            .uncompacted_bytes
            .store(read_file.get_ref().metadata()?.len(), Ordering::SeqCst);

        Ok(())
    }

    fn compact(&self) -> Result<()> {
        // Flush any pending writes before reading
        self.inner.write_state.lock()?.log_file.flush()?;

        let log = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&self.inner.path)?;

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
            .open(&self.inner.path)?;

        let mut records: Vec<&LogRecord> = commands.values().collect();
        records.sort();

        self.inner.index.clear();
        let mut write_pos = 0;
        for record in records {
            let start = write_pos;

            let bytes = bincode::encode_to_vec(&record.command, standard())?;

            log.write_all(&bytes)?;
            write_pos += bytes.len() as u64;

            // Rebuild the index with new offsets
            if let KvCommand::Set { key, value: _ } = &record.command {
                self.inner.index.insert(key.clone(), start);
            }
        }

        log.flush()?;

        self.inner.uncompacted_bytes.store(0, Ordering::SeqCst);
        self.inner
            .write_state
            .lock()?
            .write_pos
            .store(write_pos, Ordering::SeqCst);

        // Update file handles after compaction
        drop(log);
        let write_file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .append(true)
            .open(&self.inner.path)?;
        let read_file = OpenOptions::new().read(true).open(&self.inner.path)?;

        self.inner.write_state.lock()?.log_file = BufWriter::new(write_file);
        *self.inner.read_file.lock()? = BufReader::new(read_file);

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
    fn set(&self, key: String, value: String) -> Result<()> {
        let command = KvCommand::Set {
            key: key.clone(),
            value,
        };
        let bytes = bincode::encode_to_vec(command, standard())?;

        // Acquire lock
        let mut write_state = self.inner.write_state.lock()?;

        // Load position value
        let start = write_state.write_pos.load(Ordering::SeqCst);

        write_state.log_file.write_all(&bytes)?;

        // Update position
        let bytes_written = bytes.len() as u64;

        write_state
            .write_pos
            .fetch_add(bytes_written, Ordering::SeqCst);

        drop(write_state);

        self.inner.index.insert(key.clone(), start);

        self.inner
            .uncompacted_bytes
            .fetch_add(bytes_written, Ordering::SeqCst);

        // Check if compaction is needed
        if self.inner.uncompacted_bytes.load(Ordering::SeqCst) > COMPACTION_THRESHOLD {
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
    fn get(&self, key: String) -> Result<Option<String>> {
        // Flush writes to ensure read_file sees all data
        self.inner.write_state.lock()?.log_file.flush()?;

        let pointer = self.inner.index.get(&key);

        if let Some(p) = pointer {
            // Get and hold lock to avoid TOCTOU race conditions
            let mut read_file = self.inner.read_file.lock()?;
            read_file.seek(SeekFrom::Start(*p))?;
            let command: KvCommand = bincode::decode_from_reader(&mut *read_file, standard())?;
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
    fn rm(&self, key: String) -> Result<()> {
        match self.inner.index.remove(&key) {
            Some(_) => {
                let command = KvCommand::Rm { key };
                let bytes = bincode::encode_to_vec(command, standard())?;

                let mut write_state = self.inner.write_state.lock()?;
                write_state.log_file.write_all(&bytes)?;
                write_state.log_file.flush()?;

                write_state
                    .write_pos
                    .fetch_add(bytes.len() as u64, Ordering::SeqCst);
                drop(write_state);

                self.inner
                    .uncompacted_bytes
                    .fetch_add(bytes.len() as u64, Ordering::SeqCst);

                // Check if compaction is needed
                if self.inner.uncompacted_bytes.load(Ordering::SeqCst) > COMPACTION_THRESHOLD {
                    self.compact()?;
                }

                Ok(())
            }
            None => Err(KvError::Remove),
        }
    }
}
