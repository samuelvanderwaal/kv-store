mod kv;
mod sled;

pub use kv::{KvCommand, KvStore};
pub use sled::KvSled;

use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::{KvError, Result};

/// Response from the server after processing a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KvResponse {
    /// Successful operation with optional value (for Get commands)
    Ok(Option<String>),
    /// Error occurred
    Err(String),
}

pub trait KvEngine: Clone + Send + 'static {
    fn get(&self, key: String) -> Result<Option<String>>;

    fn set(&self, key: String, value: String) -> Result<()>;

    fn rm(&self, key: String) -> Result<()>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EngineType {
    Kvs,
    Sled,
}

impl FromStr for EngineType {
    type Err = KvError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "kvs" | "kvstore" => Ok(EngineType::Kvs),
            "sled" | "sld" => Ok(EngineType::Sled),
            _ => Err(KvError::InvalidEngineName),
        }
    }
}

/// A wrapper enum that can hold any engine implementation.
/// This allows dynamic dispatch while keeping the trait Clone-able.
#[derive(Clone)]
pub enum Engine {
    Kvs(KvStore),
    Sled(KvSled),
}

impl Engine {
    /// Create an Engine from an EngineType and path
    pub fn open(engine_type: EngineType, path: impl AsRef<std::path::Path>) -> Result<Self> {
        match engine_type {
            EngineType::Kvs => Ok(Engine::Kvs(KvStore::open(path.as_ref())?)),
            EngineType::Sled => Ok(Engine::Sled(KvSled::open(path.as_ref())?)),
        }
    }
}

impl KvEngine for Engine {
    fn get(&self, key: String) -> Result<Option<String>> {
        match self {
            Engine::Kvs(kvs) => kvs.get(key),
            Engine::Sled(sled) => sled.get(key),
        }
    }

    fn set(&self, key: String, value: String) -> Result<()> {
        match self {
            Engine::Kvs(kvs) => kvs.set(key, value),
            Engine::Sled(sled) => sled.set(key, value),
        }
    }

    fn rm(&self, key: String) -> Result<()> {
        match self {
            Engine::Kvs(kvs) => kvs.rm(key),
            Engine::Sled(sled) => sled.rm(key),
        }
    }
}
