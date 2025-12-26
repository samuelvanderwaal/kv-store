//#![deny(missing_docs)]

//! Key Value Storage is a primitive, in-memory database with set, get, and remove methods accessed
//! via CLI.

mod client;
mod engine;
mod error;
mod server;
pub mod thread_pool;

pub use client::KvClient;
pub use engine::{Engine, EngineType, KvCommand, KvEngine, KvResponse, KvSled, KvStore};
pub use error::KvError;
pub use server::KvServer;

/// Aliased result using the library's custom error.
pub type Result<T> = std::result::Result<T, KvError>;
