mod kv;
mod sled;

pub use kv::{KvCommand, KvStore};
pub use sled::KvSled;

use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::{KvError, Result};

pub trait KvEngine {
    fn get(&mut self, key: String) -> Result<Option<String>>;

    fn set(&mut self, key: String, value: String) -> Result<()>;

    fn rm(&mut self, key: String) -> Result<()>;
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
