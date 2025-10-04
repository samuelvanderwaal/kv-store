use std::io;

use thiserror::Error;

/// Custom error for the library to represent the various types of failures.
#[derive(Debug, Error)]
pub enum KvError {
    // Config
    #[error("invalid engine name")]
    InvalidEngineName,
    #[error("invalid addr")]
    InvalidAddr,

    // IO
    #[error("failed to open datastore at path")]
    Open(#[from] io::Error),
    #[error("remove error")]
    Remove,
    #[error("network message is too large")]
    MessageTooLarge,

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
