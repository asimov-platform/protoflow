// This is free and unencumbered software released into the public domain.

extern crate std;

use crate::{prelude::Bytes, StdioConfig, StdioError, StdioSystem, System};
use protoflow_core::{Block, BlockResult, BlockRuntime, InputPort};
use protoflow_derive::Block;
use simple_mermaid::mermaid;

/// A block that writes bytes to standard error (aka stderr).
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/sys/write_stderr.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/sys/write_stderr.seq.mmd" framed)]
///
/// # Examples
///
/// ## Using the block in a system
///
/// ```rust
/// # use protoflow_blocks::*;
/// # fn main() {
/// System::build(|s| {
///     let stdin = s.read_stdin();
///     let stderr = s.write_stderr();
///     s.connect(&stdin.output, &stderr.input);
/// });
/// # }
/// ```
///
/// ## Running the block via the CLI
///
/// ```console
/// $ protoflow execute WriteStderr < input.txt 2> output.txt
/// ```
///
#[derive(Block, Clone)]
pub struct WriteStderr {
    /// The input message stream.
    #[input]
    pub input: InputPort<Bytes>,
}

impl WriteStderr {
    pub fn new(input: InputPort<Bytes>) -> Self {
        Self { input }
    }

    pub fn with_system(system: &System) -> Self {
        use crate::SystemBuilding;
        Self::new(system.input())
    }
}

impl Block for WriteStderr {
    fn execute(&mut self, runtime: &dyn BlockRuntime) -> BlockResult {
        runtime.wait_for(&self.input)?;

        while let Some(message) = self.input.recv()? {
            let mut stderr = std::io::stderr().lock();
            std::io::Write::write_all(&mut stderr, &message)?;
        }

        self.input.close()?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl StdioSystem for WriteStderr {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        use crate::SystemBuilding;

        config.reject_any()?;

        Ok(System::build(|s| {
            let stdin = config.read_stdin(s);
            let stderr = config.write_stderr(s);
            s.connect(&stdin.output, &stderr.input);
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::WriteStderr;
    use crate::{System, SystemBuilding};

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.block(WriteStderr::new(s.input()));
        });
    }
}
