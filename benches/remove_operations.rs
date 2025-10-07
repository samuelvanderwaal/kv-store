use std::collections::HashMap;

use {
    criterion::{BenchmarkId, Criterion, criterion_group, criterion_main},
    tempfile::TempDir,
};

mod common;

pub fn remove_benchmark(c: &mut Criterion) {
    let engines = vec!["kvs", "sled"];

    for engine_name in engines {
        let temp_dir = TempDir::new().expect("unable to create temporary working directory");
        let mut store = common::create_engine(engine_name, temp_dir.path());

        // Test 1: Remove existing keys
        c.bench_with_input(
            BenchmarkId::new("remove_existing_keys", engine_name),
            &engine_name,
            |b, _| {
                b.iter(|| {
                    // Re-populate store for each iteration
                    for i in 0..100 {
                        let key = format!("key_{}", i);
                        let value = format!("value_{}", i);
                        store.set(key, value).unwrap();
                    }

                    // Remove a key
                    store.rm("key_50".to_string())
                })
            },
        );

        // Test 2: Remove same key repeatedly (overwrite scenario)
        c.bench_with_input(
            BenchmarkId::new("remove_same_key", engine_name),
            &engine_name,
            |b, _| {
                b.iter(|| {
                    // Set the key first
                    store.set("key1".to_string(), "value1".to_string()).unwrap();
                    // Then remove it
                    store.rm("key1".to_string())
                })
            },
        );

        // Test 3: Remove non-existent keys (error case)
        c.bench_with_input(
            BenchmarkId::new("remove_missing_keys", engine_name),
            &engine_name,
            |b, _| {
                let mut counter = 0;
                b.iter(|| {
                    let key = format!("missing_key_{}", counter);
                    counter += 1;
                    store.rm(key)
                })
            },
        );
    }

    // Test 4: Memory-only HashMap for comparison (baseline)
    c.bench_function("memory_only_hashmap_remove", |b| {
        b.iter(|| {
            let mut map = HashMap::new();
            for i in 0..100 {
                let key = format!("key_{}", i);
                let value = format!("value_{}", i);
                map.insert(key, value);
            }

            map.remove("key_50")
        })
    });
}

criterion_group!(
    name = benches;
    config = common::io_benchmark_config();
    targets = remove_benchmark
);
criterion_main!(benches);
