use std::{sync::mpsc, thread, time::Duration};

use crossbeam::sync::WaitGroup;
use kvs::KvClient;

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

    for thread_num in thread_nums {
        c.bench_with_input(
            BenchmarkId::new("write_queued_kvstore", thread_num),
            &thread_num,
            |b, &thread_num| {
                // ===== SETUP (outside b.iter) =====

                // 1. Create server with N threads
                let server_pool = SharedQueueThreadPool::new(thread_num).unwrap();
                let addr = SocketAddr::from_str("127.0.0.1:0").unwrap();
                let server =
                    KvServer::new(addr, temp_dir.clone(), EngineType::Kvs, server_pool).unwrap();
                let actual_addr = server.local_addr().unwrap();

                // 2. Spawn server in background
                let (shutdown_tx, shutdown_rx) = mpsc::channel();
                std::thread::spawn(move || {
                    server.run_until_shutdown(shutdown_rx).unwrap(); // Need this method
                });
                thread::sleep(Duration::from_millis(100)); // Wait for ready

                // 3. Create CLIENT thread pool (for generating load)
                // Use 1000 threads = 1 per request
                let client_pool = SharedQueueThreadPool::new(1000).unwrap();

                // 4. Pre-allocate keys/values (same every iteration!)
                let keys: Vec<String> = (0..1000).map(|i| format!("key_{:05}", i)).collect();
                let value = "value".to_string(); // Same value for all

                // ===== BENCHMARK LOOP (inside b.iter) =====
                b.iter(|| {
                    let wg = WaitGroup::new(); // One signal for all completions

                    // Spawn 1000 concurrent SET requests via client pool
                    for key in &keys {
                        let key = key.clone();
                        let value = value.clone();
                        let wg = wg.clone();
                        let addr = actual_addr;

                        client_pool.spawn(move || {
                            let mut client = KvClient::new(addr).unwrap();
                            client.set(key, value).unwrap(); // Need this method
                            // No explicit drop needed - WaitGroup drops automatically
                            drop(wg);
                        });
                    }

                    wg.wait(); // Block until all 1000 requests complete
                    // This is the throughput measurement!
                });

                // ===== TEARDOWN =====
                shutdown_tx.send(()).unwrap(); // Signal server to stop
            },
        );
    }
}
