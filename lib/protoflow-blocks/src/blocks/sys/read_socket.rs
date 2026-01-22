extern crate std;

use crate::{
    prelude::{vec, Bytes, String},
    StdioConfig, StdioError, StdioSystem, System,
};
use protoflow_core::{
    error, info, Block, BlockError, BlockResult, BlockRuntime, OutputPort, Port, PortResult,
    SystemBuilding,
};
use protoflow_derive::Block;
use serde::{Deserialize, Serialize};
use simple_mermaid::mermaid;
use std::{
    format,
    io::Read,
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex, PoisonError},
};

/// A block that reads a proto object from a TCP port.
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/sys/read_socket.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/sys/read_socket.seq.mmd" framed)]
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
/// $ protoflow execute ReadSocket connection=tcp://127.0.0.1:7077 buffer_size="1024"
/// ```
///
#[derive(Block, Clone)]
pub struct ReadSocket {
    #[output]
    pub output: OutputPort<Bytes>,
    #[parameter]
    pub config: ReadSocketConfig,
    #[state]
    pub listener: Arc<Mutex<Option<TcpListener>>>,
    #[state]
    pub stream: Arc<Mutex<Option<TcpStream>>>,
}

impl ReadSocket {
    pub fn with_params(output: OutputPort<Bytes>, config: Option<ReadSocketConfig>) -> Self {
        Self {
            output,
            config: config.unwrap_or(ReadSocketConfig {
                connection: String::from("tcp://127.0.0.1:7077"),
                buffer_size: 1024,
            }),
            listener: Arc::new(Mutex::new(None)),
            stream: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_system(system: &System, config: Option<ReadSocketConfig>) -> Self {
        Self::with_params(system.output(), config)
    }
}

impl StdioSystem for ReadSocket {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        config.allow_only(vec!["connection", "buffer_size"])?;

        let connection = config.get_string("connection")?;
        let buffer_size: usize = config.get("buffer_size")?;

        Ok(System::build(|s| {
            let stdout = config.write_stdout(s);
            let read_socket = s.block(ReadSocket::with_system(
                s,
                Some(ReadSocketConfig {
                    connection,
                    buffer_size,
                }),
            ));
            s.connect(&read_socket.output, &stdout.input);
        }))
    }
}

impl Block for ReadSocket {
    fn prepare(&mut self, _runtime: &dyn BlockRuntime) -> BlockResult {
        let listener = TcpListener::bind(&self.config.connection)?;
        *self.listener.lock().map_err(lock_error)? = Some(listener);
        info!("Server listening on {}", &self.config.connection);
        Ok(())
    }

    fn execute(&mut self, _: &dyn BlockRuntime) -> BlockResult {
        let mut stream_guard = self.stream.lock().map_err(lock_error)?;

        if stream_guard.is_none() {
            let listener_lock = self.listener.lock().map_err(lock_error)?;
            let listener = listener_lock
                .as_ref()
                .ok_or(BlockError::Other("Invalid TCP listener".into()))?;

            let (stream, addr) = listener.accept().map_err(|e| {
                error!("Failed to accept client connection: {}", e);
                BlockError::Other("Failed to accept client connection".into())
            })?;
            info!("Accepted connection from {}", addr);
            *stream_guard = Some(stream);
        }

        if let Some(stream) = stream_guard.as_mut() {
            handle_client::<_>(stream, self.config.buffer_size, |message| {
                info!("Processing received message");
                if self.output.is_connected() {
                    self.output.send(message)?;
                }
                Ok(())
            })
            .map_err(|e| {
                error!("Error handling client: {}", e);
                BlockError::Other("Error handling client".into())
            })?;
        }

        Ok(())
    }
}

fn lock_error<T>(err: PoisonError<T>) -> BlockError {
    BlockError::Other(format!("Failed to acquire lock: {:?}", err))
}

fn handle_client<F>(
    stream: &mut TcpStream,
    buffer_size: usize,
    process_fn: F,
) -> Result<(), BlockError>
where
    F: Fn(&Bytes) -> PortResult<()>,
{
    let mut buffer = vec![0; buffer_size];

    loop {
        let bytes_read = stream.read(&mut buffer)?;

        if bytes_read == 0 {
            info!("Client disconnected");
            break;
        }

        let message = Bytes::copy_from_slice(&buffer[..bytes_read]);
        info!("Received message: {:?}", message);

        if let Err(e) = process_fn(&message) {
            error!("Failed to process message: {:?}", e);
            return Err(BlockError::Other("Failed to process message".into()));
        }
    }

    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReadSocketConfig {
    pub connection: String,
    pub buffer_size: usize,
}

#[cfg(test)]
pub mod read_socket_tests {

    use protoflow_core::SystemBuilding;

    use crate::{SysBlocks, System};

    use super::ReadSocket;
    extern crate std;

    #[test]
    #[ignore]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.block(ReadSocket::with_system(s, None));
        });
    }
    #[test]
    #[ignore = "requires port"]
    fn run_server() {
        use super::*;
        use SystemBuilding;
        if let Err(e) = System::run(|s| {
            let std_out = s.write_stdout();

            let read_socket = s.block(ReadSocket::with_system(
                s,
                Some(ReadSocketConfig {
                    connection: String::from("tcp://127.0.0.1:7077"),
                    buffer_size: 512,
                }),
            ));
            s.connect(&read_socket.output, &std_out.input);
        }) {
            error!("{}", e)
        }
    }
}
