use std::thread;

use super::Result;

pub trait ThreadPool {
    fn new(threads: u32) -> Result<impl ThreadPool>;

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static;
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

pub struct SharedQueueThreadPool {}

impl ThreadPool for SharedQueueThreadPool {
    #[allow(refining_impl_trait)]
    fn new(_threads: u32) -> Result<SharedQueueThreadPool> {
        Ok(SharedQueueThreadPool {})
    }

    fn spawn<F: FnOnce() + Send + 'static>(&self, _job: F) {
        todo!();
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
