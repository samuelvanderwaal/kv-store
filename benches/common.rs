use criterion::Criterion;
use std::time::Duration;

use kvs::{KvEngine, KvSled, KvStore};

/// Shared benchmark configuration for I/O-heavy operations
///
/// This configuration is optimized for disk I/O benchmarks which have:
/// - High variance due to OS scheduling, file system caching, etc.
/// - Need for longer warm-up to stabilize I/O patterns
/// - Higher noise thresholds due to inherent I/O variability
pub fn io_benchmark_config() -> Criterion {
    Criterion::default()
        .sample_size(1000) // More samples for better statistics
        .measurement_time(Duration::from_secs(15)) // Longer measurement time
        .warm_up_time(Duration::from_secs(5)) // Longer warm-up for I/O
        .noise_threshold(0.08) // Realistic noise threshold for I/O (8%)
}

/// Creates a storage engine by name for benchmarking
///
/// Supports "kvs" and "sled" engines. This helper function allows
/// benchmarks to be parameterized across different storage backends.
pub fn create_engine(engine_name: &str, path: &std::path::Path) -> Box<dyn KvEngine> {
    match engine_name {
        "kvs" => Box::new(KvStore::open(path).unwrap()),
        "sled" => Box::new(KvSled::open(path).unwrap()),
        _ => panic!("Unknown engine: {}", engine_name),
    }
}
