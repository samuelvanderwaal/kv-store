use std::{
    fs::{self, File},
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc, RwLock,
        mpsc::{Receiver, TryRecvError},
    },
    thread,
    time::Duration,
};

use crate::{
    Engine, EngineType, KvCommand, KvEngine, KvError, KvResponse, Result, thread_pool::ThreadPool,
};
use bincode::config::standard;

pub struct KvServer<P: ThreadPool> {
    engine: Arc<RwLock<Engine>>,
    thread_pool: P,
    listener: TcpListener,
}

impl<P: ThreadPool> KvServer<P> {
    pub fn new(
        addr: SocketAddr,
        storage_path: PathBuf,
        engine_type: EngineType,
        thread_pool: P,
    ) -> Result<KvServer<P>> {
        let engine = Arc::new(RwLock::new(open_engine(&storage_path, engine_type)?));
        let listener = TcpListener::bind(addr)?;

        Ok(KvServer {
            engine,
            thread_pool,
            listener,
        })
    }

    pub fn run(&self) -> Result<()> {
        for stream in self.listener.incoming() {
            let stream = stream?;
            let engine = self.engine.clone();
            self.thread_pool.spawn(move || {
                let _ = handle_connection(stream, engine);
            });
        }
        Ok(())
    }

    pub fn run_until_shutdown(&self, shutdown_rx: Receiver<()>) -> Result<()> {
        self.listener.set_nonblocking(true)?;

        loop {
            match shutdown_rx.try_recv() {
                Ok(()) => break,
                Err(TryRecvError::Disconnected) => break,
                Err(TryRecvError::Empty) => (),
            }

            // Try to accept connection
            match self.listener.accept() {
                Ok((stream, _)) => {
                    let engine = self.engine.clone();
                    self.thread_pool.spawn(move || {
                        let _ = handle_connection(stream, engine);
                    });
                }
                Err(ref e) => {
                    if e.kind() != std::io::ErrorKind::WouldBlock {
                        // No connection waiting, sleep briefly
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        }

        Ok(())
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }
}

fn get_engine_type(p: &Path) -> Result<Option<EngineType>> {
    let engine_file = p.join(".engine.lock");
    if !engine_file.exists() {
        return Ok(None);
    }

    Ok(Some(serde_json::from_reader(File::open(engine_file)?)?))
}
fn set_engine_type(p: &Path, engine: EngineType) -> Result<()> {
    let engine_file = p.join(".engine.lock");
    fs::write(engine_file, serde_json::to_string(&engine)?)?;

    Ok(())
}
/// Opens the appropriate engine based on what's persisted, or creates new with specified type
pub fn open_engine(path: &Path, requested_engine: EngineType) -> Result<Engine> {
    match get_engine_type(path)? {
        Some(existing_engine) if existing_engine != requested_engine => {
            return Err(KvError::WrongEngine {
                requested: requested_engine,
                existing: existing_engine,
            });
        }
        None => {
            // First time, persist the engine choice
            set_engine_type(path, requested_engine)?;
        }
        _ => {} // Engine types match, proceed
    }

    Engine::open(requested_engine, path)
}

fn handle_connection(mut stream: TcpStream, engine: Arc<RwLock<Engine>>) -> Result<()> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes) as usize;

    // Vec w/ len and capacity of `len`.
    let mut data = vec![0u8; len];
    stream.read_exact(&mut data)?;

    let (command, _): (KvCommand, usize) = bincode::serde::decode_from_slice(&data, standard())?;

    // Process command and create response
    let response = match command {
        KvCommand::Get { key } => {
            let engine = engine.read()?;
            match engine.get(key) {
                Ok(value) => KvResponse::Ok(value),
                Err(e) => KvResponse::Err(e.to_string()),
            }
        }
        KvCommand::Set { key, value } => {
            let engine = engine.read()?;
            match engine.set(key, value) {
                Ok(()) => KvResponse::Ok(None),
                Err(e) => KvResponse::Err(e.to_string()),
            }
        }
        KvCommand::Rm { key } => {
            let engine = engine.read()?;
            match engine.rm(key) {
                Ok(()) => KvResponse::Ok(None),
                Err(e) => KvResponse::Err(e.to_string()),
            }
        }
    };

    // Serialize and send response back to client
    let response_bytes = bincode::serde::encode_to_vec(&response, standard())?;
    let response_len = response_bytes.len() as u32;

    stream.write_all(&response_len.to_be_bytes())?;
    stream.write_all(&response_bytes)?;
    stream.flush()?;

    Ok(())
}
