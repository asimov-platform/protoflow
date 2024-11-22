// This is free and unencumbered software released into the public domain.

extern crate std;

use crate::{prelude::Bytes, StdioConfig, StdioError, StdioSystem, System};
use protoflow_core::{Block, BlockResult, BlockRuntime, InputPort};
use protoflow_derive::Block;
use simple_mermaid::mermaid;

/// A block that writes bytes to standard output (aka stdout).
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/sys/write_stdout.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/sys/write_stdout.seq.mmd" framed)]
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
///     let stdout = s.write_stdout();
///     s.connect(&stdin.output, &stdout.input);
/// });
/// # }
/// ```
///
/// ## Running the block via the CLI
///
/// ```console
/// $ protoflow execute WriteStdout < input.txt > output.txt
/// ```
///
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

    pub fn with_system(system: &System) -> Self {
        use crate::SystemBuilding;
        Self::new(system.input())
    }
}

impl Block for WriteStdout {
    fn execute(&mut self, runtime: &dyn BlockRuntime) -> BlockResult {
        runtime.wait_for(&self.input)?;

        while let Some(message) = self.input.recv()? {
            let mut stdout = std::io::stdout().lock();
            std::io::Write::write_all(&mut stdout, &message)?;
        }

        Ok(())
    }
}

#[cfg(feature = "std")]
impl StdioSystem for WriteStdout {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        use crate::SystemBuilding;

        config.reject_any()?;

        Ok(System::build(|s| {
            let stdin = config.read_stdin(s);
            let stdout = config.write_stdout(s);
            s.connect(&stdin.output, &stdout.input);
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::WriteStdout;
    use crate::{System, SystemBuilding};

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.block(WriteStdout::new(s.input()));
        });
    }
}
