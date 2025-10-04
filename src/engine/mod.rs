mod kv;
mod sled;

pub use kv::{KvCommand, KvStore};
pub use sled::KvSled;

use crate::Result;

pub trait KvEngine {
    fn get(&mut self, key: String) -> Result<Option<String>>;

    fn set(&mut self, key: String, value: String) -> Result<()>;

    fn rm(&mut self, key: String) -> Result<()>;
}
