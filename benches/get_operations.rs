use std::collections::HashMap;

use {
    criterion::{BenchmarkId, Criterion, criterion_group, criterion_main},
    tempfile::TempDir,
};

mod common;

use common::*;

pub fn get_benchmark(c: &mut Criterion) {
    let engines = vec!["kvs", "sled"];

    for engine_name in engines {
        let temp_dir = TempDir::new().expect("unable to create temporary working directory");
        let store = common::create_engine(engine_name, temp_dir.path());

        // Pre-populate the store with data
        for i in 0..1000 {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            store.set(key, value).unwrap();
        }

        // Test 1: Get existing keys (cache hit scenario)
        c.bench_with_input(
            BenchmarkId::new("get_existing_keys", engine_name),
            &engine_name,
            |b, _| {
                // Pre-allocate keys to avoid measuring format!() overhead
                let keys: Vec<String> = (0..1000).map(|i| format!("key_{}", i)).collect();
                let mut counter = 0;
                b.iter(|| {
                    let key = keys[counter % 1000].clone();
                    counter += 1;
                    store.get(key)
                })
            },
        );

        // Test 2: Get same key repeatedly (best case scenario)
        c.bench_with_input(
            BenchmarkId::new("get_same_key", engine_name),
            &engine_name,
            |b, _| b.iter(|| store.get("key_500".to_string())),
        );

        // Test 3: Get non-existent keys (worst case scenario)
        c.bench_with_input(
            BenchmarkId::new("get_missing_keys", engine_name),
            &engine_name,
            |b, _| {
                // Pre-allocate keys to avoid measuring format!() overhead
                let keys: Vec<String> = (0..100000).map(|i| format!("missing_key_{}", i)).collect();
                let mut counter = 0;
                b.iter(|| {
                    let key = keys[counter % 100000].clone();
                    counter += 1;
                    store.get(key)
                })
            },
        );
    }

    // Test 4: Memory-only HashMap for comparison (baseline)
    c.bench_function("memory_only_hashmap_get", |b| {
        let mut map = HashMap::new();
        for i in 0..1000 {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            map.insert(key, value);
        }

        // Pre-allocate keys to avoid measuring format!() overhead
        let keys: Vec<String> = (0..1000).map(|i| format!("key_{}", i)).collect();
        let mut counter = 0;
        b.iter(|| {
            let key = &keys[counter % 1000];
            counter += 1;
            map.get(key)
        })
    });
}

criterion_group!(
    name = benches;
    config = common::io_benchmark_config();
    targets = get_benchmark
);
criterion_main!(benches);
