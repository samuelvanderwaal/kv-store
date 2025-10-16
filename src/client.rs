use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
};

use bincode::config::standard;

use crate::{KvCommand, KvResponse, Result};

pub struct KvClient {
    stream: TcpStream,
}

impl KvClient {
    pub fn new(addr: SocketAddr) -> Result<Self> {
        let stream = TcpStream::connect(addr)?;

        Ok(KvClient { stream })
    }

    /// Create a client from an existing stream (useful for testing)
    pub fn from_stream(stream: TcpStream) -> Self {
        KvClient { stream }
    }

    pub fn send_command(&mut self, command: KvCommand) -> Result<()> {
        let bytes = bincode::serde::encode_to_vec(&command, standard())?;
        let len = bytes.len() as u32;

        // Send command
        self.stream.write_all(&len.to_be_bytes())?;
        self.stream.write_all(&bytes)?;
        self.stream.flush()?;

        // Read response
        let mut response_len_bytes = [0u8; 4];
        self.stream.read_exact(&mut response_len_bytes)?;
        let response_len = u32::from_be_bytes(response_len_bytes) as usize;

        let mut response_data = vec![0u8; response_len];
        self.stream.read_exact(&mut response_data)?;

        let (response, _): (KvResponse, usize) =
            bincode::serde::decode_from_slice(&response_data, standard())?;

        // Handle response
        match response {
            KvResponse::Ok(Some(value)) => {
                println!("{}", value);
            }
            KvResponse::Ok(None) => {
                // For Get commands with no value, print "Key not found"
                // For Set/Rm commands, no output expected on success
                if matches!(command, KvCommand::Get { .. }) {
                    println!("Key not found");
                }
            }
            KvResponse::Err(err) => {
                eprintln!("{}", err);
                std::process::exit(1);
            }
        }

        Ok(())
    }

    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        self.send_command(KvCommand::Set { key, value })
    }
}
