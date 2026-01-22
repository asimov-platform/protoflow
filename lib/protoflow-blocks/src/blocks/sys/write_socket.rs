extern crate std;

use super::SysBlocks;
use crate::{
    prelude::{bytes::Bytes, vec, String},
    StdioConfig, StdioError, StdioSystem, System,
};
use protoflow_core::{
    error, Block, BlockError, BlockResult, BlockRuntime, InputPort, SystemBuilding,
};
use protoflow_derive::Block;
use serde::{Deserialize, Serialize};
use simple_mermaid::mermaid;
use std::{
    format,
    net::TcpStream,
    sync::{Arc, Mutex, PoisonError},
};

/// A block that writes a proto object to a TCP socket.
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/sys/write_socket.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/sys/write_socket.seq.mmd" framed)]
///
/// # Examples
///
/// ## Using the block in a system
///
/// ```rust
/// # use protoflow_blocks::*;
/// # fn main() {
/// System::build(|s| {
///     // TODO
/// });
/// # }
/// ```
///
/// ## Running the block via the CLI
///
/// ```console
/// $ protoflow execute WriteSocket connection=tcp://127.0.0.1:7077 buffer_size="1024"
/// ```
///
#[derive(Block, Clone)]
pub struct WriteSocket {
    #[output]
    pub input: InputPort<Bytes>,
    #[parameter]
    pub config: WriteSocketConfig,
    #[state]
    pub stream: Arc<Mutex<Option<TcpStream>>>,
}

impl WriteSocket {
    pub fn with_params(input: InputPort<Bytes>, config: Option<WriteSocketConfig>) -> Self {
        Self {
            input,
            config: config.unwrap_or(WriteSocketConfig {
                connection: String::from("tcp://127.0.0.1:7077"),
                buffer_size: 1024,
            }),
            stream: Arc::new(Mutex::new(None)),
        }
    }
    pub fn with_config(self, config: WriteSocketConfig) -> Self {
        Self { config, ..self }
    }
    pub fn with_system(system: &System, config: Option<WriteSocketConfig>) -> Self {
        Self::with_params(system.input(), config)
    }
}

impl StdioSystem for WriteSocket {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        config.allow_only(vec!["connection", "buffer_size"])?;

        let connection = config.get_string("connection")?;
        let buffer_size: usize = config.get("buffer_size")?;

        Ok(System::build(|s| {
            let stdin = config.read_stdin(s);
            let write_socket = s.write_socket().with_config(WriteSocketConfig {
                connection,
                buffer_size,
            });

            s.connect(&stdin.output, &write_socket.input);
        }))
    }
}

impl Block for WriteSocket {
    fn execute(&mut self, _: &dyn BlockRuntime) -> BlockResult {
        while let Some(input) = self.input.recv()? {
            let mut stream_guard = self.stream.lock().map_err(lock_error)?;

            if stream_guard.is_none() {
                *stream_guard = Some(TcpStream::connect(&self.config.connection).map_err(|e| {
                    error!("Failed to connect to {}: {}", &self.config.connection, e);
                    BlockError::Other(format!(
                        "Failed to connect to {}: {}",
                        &self.config.connection, e
                    ))
                })?);
            }

            let stream = stream_guard.as_mut().ok_or_else(|| {
                error!("Stream is not connected");
                BlockError::Other("Stream is not connected".into())
            })?;

            std::io::Write::write_all(stream, &input)?;
        }
        Ok(())
    }
}

fn lock_error<T>(err: PoisonError<T>) -> BlockError {
    BlockError::Other(format!("Failed to acquire lock: {:?}", err))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WriteSocketConfig {
    pub connection: String,
    pub buffer_size: usize,
}

#[cfg(test)]
pub mod write_socket_tests {
    use super::WriteSocket;
    use crate::System;
    use protoflow_core::SystemBuilding;
    extern crate std;

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.block(WriteSocket::with_system(s, None));
        });
    }
    #[test]
    #[ignore = "requires port"]
    fn run_client() {
        use super::*;
        use protoflow_core::SystemBuilding;
        if let Err(e) = System::run(|s| {
            let stdin = s.read_stdin();

            let write_socket = s.write_socket().with_config(WriteSocketConfig {
                connection: String::from("tcp://127.0.0.1:7077"),
                buffer_size: 512,
            });
            s.connect(&stdin.output, &write_socket.input);
        }) {
            error!("{}", e)
        }
    }
}
