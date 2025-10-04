use std::io;

use thiserror::Error;

use crate::engine::EngineType;

/// Custom error for the library to represent the various types of failures.
#[derive(Debug, Error)]
pub enum KvError {
    // Config
    #[error("invalid engine name")]
    InvalidEngineName,
    #[error("invalid addr")]
    InvalidAddr,
    #[error("failed to build config")]
    Config(#[from] config::ConfigError),
    #[error("Wrong engine: requested {requested:?} but data was created with {existing:?}")]
    WrongEngine {
        requested: EngineType,
        existing: EngineType,
    },

    // IO
    #[error("failed to open file at path")]
    Open(#[from] io::Error),
    #[error("remove error")]
    Remove,
    #[error("failed to deserialize")]
    Deserialize(#[from] serde_json::Error),

    // Encoding
    #[error("bson error")]
    Bson(#[from] bson::error::Error),
    #[error("bincode encoding error")]
    BincodeEncode(#[from] bincode::error::EncodeError),
    #[error("bincode decoding error")]
    BincodeDecoding(#[from] bincode::error::DecodeError),

    // Index
    #[error("failed to insert into index")]
    IndexInsertion,
}
