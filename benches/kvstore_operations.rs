use std::{collections::HashMap, time::Duration};

use {
    criterion::{Criterion, criterion_group, criterion_main},
    tempfile::TempDir,
};

use kvs::KvStore;

pub fn insert_benchmark(c: &mut Criterion) {
    let temp_dir = TempDir::new().expect("unable to create temporary working directory");
    let mut store = KvStore::open(temp_dir.path()).unwrap();

    // Test 1: Unique keys (realistic insert scenario)
    c.bench_function("insert_unique_keys", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter);
            let value = format!("value_{}", counter);
            counter += 1;
            store.set(key, value)
        })
    });

    // Test 2: Same key overwrites (your current test)
    c.bench_function("insert_same_key", |b| {
        b.iter(|| store.set("key1".to_string(), "value1".to_string()))
    });

    // Test 3: Mixed operations (more realistic workload)
    c.bench_function("mixed_operations", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter % 100); // Cycle through 100 keys
            let value = format!("value_{}", counter);
            counter += 1;
            store.set(key, value)
        })
    });

    c.bench_function("memory_only_hashmap", |b| {
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

pub fn remove_benchmark(c: &mut Criterion) {
    let temp_dir = TempDir::new().expect("unable to create temporary working directory");
    let mut store = KvStore::open(temp_dir.path()).unwrap();

    // Test 1: Remove existing keys
    c.bench_function("remove_existing_keys", |b| {
        b.iter(|| {
            // Re-populate store for each iteration
            for i in 0..100 {
                let key = format!("key_{}", i);
                let value = format!("value_{}", i);
                store.set(key, value).unwrap();
            }

            // Remove a key
            store.remove("key_50".to_string())
        })
    });

    // Test 2: Remove same key repeatedly (overwrite scenario)
    c.bench_function("remove_same_key", |b| {
        b.iter(|| {
            // Set the key first
            store.set("key1".to_string(), "value1".to_string()).unwrap();
            // Then remove it
            store.remove("key1".to_string())
        })
    });

    // Test 3: Remove non-existent keys (error case)
    c.bench_function("remove_missing_keys", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("missing_key_{}", counter);
            counter += 1;
            store.remove(key)
        })
    });

    // Test 4: Memory-only HashMap for comparison
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
    config = Criterion::default()
        .sample_size(500)
        .measurement_time(Duration::from_secs(5));
    targets = insert_benchmark, get_benchmark, remove_benchmark
);
criterion_main!(benches);
