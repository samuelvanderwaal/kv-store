use std::collections::HashMap;

use {
    criterion::{BenchmarkId, Criterion, criterion_group, criterion_main},
    tempfile::TempDir,
};

mod common;

pub fn set_benchmark(c: &mut Criterion) {
    let engines = vec!["kvs", "sled"];

    for engine_name in engines {
        let temp_dir = TempDir::new().expect("unable to create temporary working directory");
        let mut store = common::create_engine(engine_name, temp_dir.path());

        // Test 1: Unique keys (realistic insert scenario)
        c.bench_with_input(
            BenchmarkId::new("set_unique_keys", engine_name),
            &engine_name,
            |b, _| {
                let mut counter = 0;
                b.iter(|| {
                    let key = format!("key_{}", counter);
                    let value = format!("value_{}", counter);
                    counter += 1;
                    store.set(key, value)
                })
            },
        );

        // Test 2: Same key overwrites
        c.bench_with_input(
            BenchmarkId::new("set_same_key", engine_name),
            &engine_name,
            |b, _| b.iter(|| store.set("key1".to_string(), "value1".to_string())),
        );

        // Test 3: Mixed operations (more realistic workload)
        c.bench_with_input(
            BenchmarkId::new("set_mixed_operations", engine_name),
            &engine_name,
            |b, _| {
                let mut counter = 0;
                b.iter(|| {
                    let key = format!("key_{}", counter % 100); // Cycle through 100 keys
                    let value = format!("value_{}", counter);
                    counter += 1;
                    store.set(key, value)
                })
            },
        );
    }

    // Test 4: Memory-only HashMap for comparison (baseline)
    c.bench_function("memory_only_hashmap_set", |b| {
        let mut map = HashMap::new();
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter);
            let value = format!("value_{}", counter);
            counter += 1;
            map.insert(key, value);
        })
    });
}

criterion_group!(
    name = benches;
    config = common::io_benchmark_config();
    targets = set_benchmark
);
criterion_main!(benches);
