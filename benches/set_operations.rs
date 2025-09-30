use std::collections::HashMap;

use {
    criterion::{Criterion, criterion_group, criterion_main},
    tempfile::TempDir,
};

use kvs::KvStore;

mod common;

pub fn set_benchmark(c: &mut Criterion) {
    let temp_dir = TempDir::new().expect("unable to create temporary working directory");
    let mut store = KvStore::open(temp_dir.path()).unwrap();

    // Test 1: Unique keys (realistic insert scenario)
    c.bench_function("set_unique_keys", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter);
            let value = format!("value_{}", counter);
            counter += 1;
            store.set(key, value)
        })
    });

    // Test 2: Same key overwrites
    c.bench_function("set_same_key", |b| {
        b.iter(|| store.set("key1".to_string(), "value1".to_string()))
    });

    // Test 3: Mixed operations (more realistic workload)
    c.bench_function("set_mixed_operations", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter % 100); // Cycle through 100 keys
            let value = format!("value_{}", counter);
            counter += 1;
            store.set(key, value)
        })
    });

    // Test 4: Memory-only HashMap for comparison
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
