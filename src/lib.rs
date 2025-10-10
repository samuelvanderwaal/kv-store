//#![deny(missing_docs)]

//! Key Value Storage is a primitive, in-memory database with set, get, and remove methods accessed
//! via CLI.

mod engine;
mod error;

pub use engine::{EngineType, KvCommand, KvEngine, KvResponse, KvSled, KvStore};
pub use error::KvError;

/// Aliased result using the library's custom error.
pub type Result<T> = std::result::Result<T, KvError>;
