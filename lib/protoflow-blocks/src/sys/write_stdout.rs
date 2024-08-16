// This is free and unencumbered software released into the public domain.

extern crate std;

use protoflow_core::{prelude::Bytes, Block, BlockResult, BlockRuntime, InputPort};
use protoflow_derive::Block;

/// A block that writes bytes to standard output (aka stdout).
#[derive(Block, Clone)]
pub struct WriteStdout {
    /// The input message stream.
    #[input]
    pub input: InputPort<Bytes>,
}

impl WriteStdout {
    pub fn new(input: InputPort<Bytes>) -> Self {
        Self { input }
    }
}

impl Block for WriteStdout {
    fn execute(&mut self, runtime: &dyn BlockRuntime) -> BlockResult {
        let mut stdout = std::io::stdout().lock();

        runtime.wait_for(&self.input)?;

        while let Some(message) = self.input.recv()? {
            std::io::Write::write_all(&mut stdout, &message)?;
        }

        self.input.close()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::WriteStdout;
    use protoflow_core::{transports::MockTransport, System};

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::<MockTransport>::build(|s| {
            let _ = s.block(WriteStdout::new(s.input()));
        });
    }
}