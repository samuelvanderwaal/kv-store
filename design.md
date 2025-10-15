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

## BSON --> Bincode performance

Switching to bincode improved performance significantly, likely due to both the smaller size of
bincode and the extra serialization step in BSON where we used the Document format.

| Operation                  | Median Time (µs/ns) |          Δ % | Result                  |
| -------------------------- | ------------------: | -----------: | ----------------------- |
| `set_unique_keys/kvs`      |           2.5546 µs | **-86.191%** | ✅ Performance improved |
| `set_same_key/kvs`         |           1.5418 µs | **-91.153%** | ✅ Performance improved |
| `set_mixed_operations/kvs` |           2.4443 µs | **-86.213%** | ✅ Performance improved |
| `remove_existing_keys/kvs` |           21.837 µs | **-88.558%** | ✅ Performance improved |
| `remove_same_key/kvs`      |           977.16 ns | **-78.283%** | ✅ Performance improved |
| `remove_missing_keys/kvs`  |           34.152 µs | **-50.435%** | ✅ Performance improved |
| `get_existing_keys/kvs`    |           869.67 ns | **-41.358%** | ✅ Performance improved |
| `get_same_key/kvs`         |           908.55 ns | **-36.601%** | ✅ Performance improved |
| `get_missing_keys/kvs`     |           33.754 µs | **-47.258%** | ✅ Performance improved |
