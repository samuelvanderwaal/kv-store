use std::collections::HashMap;

use {
    criterion::{Criterion, criterion_group, criterion_main},
    tempfile::TempDir,
};

use kvs::KvStore;

mod common;

pub fn get_benchmark(c: &mut Criterion) {
    let temp_dir = TempDir::new().expect("unable to create temporary working directory");
    let mut store = KvStore::open(temp_dir.path()).unwrap();

    // Pre-populate the store with data
    for i in 0..1000 {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        store.set(key, value).unwrap();
    }

    // Test 1: Get existing keys (cache hit scenario)
    c.bench_function("get_existing_keys", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter % 1000);
            counter += 1;
            store.get(key)
        })
    });

    // Test 2: Get same key repeatedly (best case scenario)
    c.bench_function("get_same_key", |b| {
        b.iter(|| store.get("key_500".to_string()))
    });

    // Test 3: Get non-existent keys (worst case scenario)
    c.bench_function("get_missing_keys", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("missing_key_{}", counter);
            counter += 1;
            store.get(key)
        })
    });

    // Test 4: Memory-only HashMap for comparison
    c.bench_function("memory_only_hashmap_get", |b| {
        let mut map = HashMap::new();
        for i in 0..1000 {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            map.insert(key, value);
        }

        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter % 1000);
            counter += 1;
            map.get(&key)
        })
    });
}

criterion_group!(
    name = benches;
    config = common::io_benchmark_config();
    targets = get_benchmark
);
criterion_main!(benches);
