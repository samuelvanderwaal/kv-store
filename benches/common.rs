use criterion::Criterion;
use std::time::Duration;

#[allow(unused_imports)]
pub use kvs::{Engine, EngineType, KvEngine};

/// Shared benchmark configuration for I/O-heavy operations
///
/// This configuration is optimized for disk I/O benchmarks which have:
/// - High variance due to OS scheduling, file system caching, etc.
/// - Need for longer warm-up to stabilize I/O patterns
/// - Higher noise thresholds due to inherent I/O variability
#[allow(dead_code)]
pub fn io_benchmark_config() -> Criterion {
    Criterion::default()
        .sample_size(1000) // More samples for better statistics
        .measurement_time(Duration::from_secs(17)) // Longer measurement time
        .warm_up_time(Duration::from_secs(5)) // Longer warm-up for I/O
        .noise_threshold(0.08) // Realistic noise threshold for I/O (8%)
}

/// Creates a storage engine by name for benchmarking
///
/// Supports "kvs" and "sled" engines. This helper function allows
/// benchmarks to be parameterized across different storage backends.
#[allow(dead_code)]
pub fn create_engine(engine_name: &str, path: &std::path::Path) -> Engine {
    match engine_name {
        "kvs" => Engine::open(EngineType::Kvs, path).unwrap(),
        "sled" => Engine::open(EngineType::Sled, path).unwrap(),
        _ => panic!("Unknown engine: {}", engine_name),
    }
}
