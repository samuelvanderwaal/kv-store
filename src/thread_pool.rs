use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
    thread,
    time::Duration,
};

use crossbeam::queue::ArrayQueue;

use super::Result;

pub trait ThreadPool {
    fn new(threads: u32) -> Result<impl ThreadPool>;

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static;
}

pub enum ThreadPoolMessage {
    RunJob(Box<dyn FnOnce() + Send + 'static>),
    Shutdown,
}

pub struct NaiveThreadPool {}

impl ThreadPool for NaiveThreadPool {
    #[allow(refining_impl_trait)]
    fn new(_threads: u32) -> Result<NaiveThreadPool> {
        Ok(NaiveThreadPool {})
    }

    fn spawn<F: FnOnce() + Send + 'static>(&self, job: F) {
        thread::spawn(job);
    }
}

const QUEUE_SIZE: usize = 1024;

pub struct SharedQueueThreadPool {
    queue: Arc<ArrayQueue<ThreadPoolMessage>>,
    pool: Vec<thread::JoinHandle<()>>,
}

impl ThreadPool for SharedQueueThreadPool {
    #[allow(refining_impl_trait)]
    fn new(threads: u32) -> Result<SharedQueueThreadPool> {
        let queue = Arc::new(ArrayQueue::new(QUEUE_SIZE));
        let mut pool = Vec::with_capacity(threads as usize);

        for _ in 0..threads {
            let queue = queue.clone();
            let handle = thread::spawn(move || {
                loop {
                    match queue.pop() {
                        Some(message) => match message {
                            ThreadPoolMessage::RunJob(job) => {
                                // Swallow panics to avoid crashing the thread.
                                let _ = catch_unwind(AssertUnwindSafe(job));
                            }
                            ThreadPoolMessage::Shutdown => break,
                        },
                        None => thread::sleep(Duration::from_micros(100)),
                    }
                }
            });
            pool.push(handle);
        }

        Ok(SharedQueueThreadPool { queue, pool })
    }

    fn spawn<F: FnOnce() + Send + 'static>(&self, job: F) {
        if self
            .queue
            .push(ThreadPoolMessage::RunJob(Box::new(job)))
            .is_err()
        {
            panic!("queue is full");
        }
    }
}

impl Drop for SharedQueueThreadPool {
    fn drop(&mut self) {
        // Push shutdown messages to queue to start shutting down threads.
        for _ in 0..self.pool.len() {
            // Ignore errors for now.
            let _ = self.queue.push(ThreadPoolMessage::Shutdown);
        }

        for handle in self.pool.drain(..) {
            let _ = handle.join(); // Ignore errors for now.
        }
    }
}

pub struct RayonThreadPool {}

impl ThreadPool for RayonThreadPool {
    #[allow(refining_impl_trait)]
    fn new(_threads: u32) -> Result<RayonThreadPool> {
        Ok(RayonThreadPool {})
    }

    fn spawn<F: FnOnce() + Send + 'static>(&self, _job: F) {
        todo!();
    }
}
