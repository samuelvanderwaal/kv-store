# BSON

The `bson` crate was chosen for serialization as a trade-off between performance and flexibility. It
provides a binary representation of JSON-like documents that is efficient for storage and
transmission.

# Tracing

The `tracing` crate was chosen for logging as it's currently canonical Rust logging framework. It
provides a flexible and extensible way to instrument and trace the execution of Rust programs. It's
excellent async support builds in the future ability to instrument this project if it's written to
use async and multi-threaded Rust.

# Network Protocol

Utilizing a simple binary format for performance and efficiency. Length prefix and then the bincode
serialization representation of the KvCommand enum. The lib has an optional "bincode" feature that
enables the Encode and Decode derive macros on the KvCommand enum for additional performance.
