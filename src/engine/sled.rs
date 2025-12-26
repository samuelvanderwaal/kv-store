use std::path::Path;

use super::KvEngine;
use crate::{KvError, Result};

use sled::{self, Db};

#[derive(Clone)]
pub struct KvSled(Db);

impl KvSled {
    /// Opens the Sled database at the given path
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(KvSled(db))
    }
}

impl KvEngine for KvSled {
    fn get(&self, key: String) -> Result<Option<String>> {
        let value = self
            .0
            .get(key)?
            .map(|ivec| ivec.to_vec())
            .map(String::from_utf8)
            .transpose()?;

        Ok(value)
    }

    fn set(&self, key: String, value: String) -> Result<()> {
        self.0.insert(key, value.into_bytes())?;
        self.0.flush()?;
        Ok(())
    }

    fn rm(&self, key: String) -> Result<()> {
        self.0.remove(key)?.ok_or(KvError::Remove)?;
        self.0.flush()?;
        Ok(())
    }
}
