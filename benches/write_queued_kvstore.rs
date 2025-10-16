use {
    criterion::{BenchmarkId, Criterion},
    kvs::{
        EngineType, KvServer,
        thread_pool::{SharedQueueThreadPool, ThreadPool},
    },
    std::{net::SocketAddr, str::FromStr},
    tempfile::TempDir,
};

mod common;

const NUM_KEYS: u32 = 1_000;

pub fn write_benchmark(c: &mut Criterion) {
    let temp_dir = TempDir::new()
        .expect("unable to create temporary working directory")
        .path()
        .to_owned();

    let cpus = num_cpus::get();

    let mut thread_nums = vec![1u32];

    let mut t = 2;
    while t < cpus * 2 {
        thread_nums.push(t as u32);
        t += 2;
    }

    let engine_type = EngineType::Kvs;
    let socket_addr = SocketAddr::from_str("[127.0.0.1]:2000").unwrap();

    for thread_num in vec![1, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20] {
        c.bench_with_input(
            BenchmarkId::new("write_kvstore_with_threads", thread_num),
            &thread_num,
            |b, _| {
                let thread_pool = SharedQueueThreadPool::new(thread_num).unwrap();
                let server =
                    KvServer::new(socket_addr, temp_dir.clone(), engine_type, thread_pool).unwrap();

                // Pre-allocate keys and values to avoid measuring format!() overhead
                let keys: Vec<String> = (0..NUM_KEYS).map(|i| format!("key_{}", i)).collect();
                let values: Vec<String> = (0..NUM_KEYS).map(|i| format!("value_{}", i)).collect();

                b.iter(|| {
                    todo!();
                })
            },
        );
    }
}
