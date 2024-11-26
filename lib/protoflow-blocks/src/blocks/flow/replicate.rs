// This is free and unencumbered software released into the public domain.

use crate::{FlowBlocks, StdioConfig, StdioError, StdioSystem, SysBlocks, System};
use protoflow_core::{
    types::Any, Block, BlockResult, BlockRuntime, InputPort, Message, OutputPort,
};
use protoflow_derive::Block;
use simple_mermaid::mermaid;

/// Duplicates a single input message stream into multiple identical output streams.
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/flow/replicate.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/flow/replicate.seq.mmd" framed)]
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
///
///     let replicate = s.replicate();
///     s.connect(&stdin.output, &replicate.input);
///
///     let stdout_1 = s.write_stdout();
///     s.connect(&replicate.output_1, &stdout_1.input);
///
///     let stdout_2 = s.write_stdout();
///     s.connect(&replicate.output_2, &stdout_2.input);
/// });
/// # }
/// ```
///
/// ## Running the block via the CLI
///
/// ```console
/// $ protoflow execute Replicate
/// ```
///
#[derive(Block, Clone)]
pub struct Replicate<T: Message = Any> {
    /// The input message stream.
    #[input]
    pub input: InputPort<T>,
    #[output]
    pub output_1: OutputPort<T>,
    #[output]
    pub output_2: OutputPort<T>,
}

impl<T: Message> Replicate<T> {
    pub fn new(input: InputPort<T>, output_1: OutputPort<T>, output_2: OutputPort<T>) -> Self {
        Self {
            input,
            output_1,
            output_2,
        }
    }
}
impl<T: Message + 'static> Replicate<T> {
    pub fn with_system(system: &System) -> Self {
        use crate::SystemBuilding;
        Self::new(system.input(), system.output(), system.output())
    }
}

impl<T: Message> Block for Replicate<T> {
    fn execute(&mut self, runtime: &dyn BlockRuntime) -> BlockResult {
        runtime.wait_for(&self.output_1)?;
        runtime.wait_for(&self.output_2)?;

        while let Some(message) = self.input.recv()? {
            self.output_1.send(&message)?;
            self.output_2.send(&message)?;
        }

        Ok(())
    }
}

#[cfg(feature = "std")]
impl<T: Message> StdioSystem for Replicate<T> {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        use crate::SystemBuilding;
        config.reject_any()?;

        Ok(System::build(|s| {
            let stdin = s.read_stdin();

            let replicate = s.replicate();
            s.connect(&stdin.output, &replicate.input);

            let stdout_1 = s.write_stdout();
            s.connect(&replicate.output_1, &stdout_1.input);

            let stdout_2 = s.write_stdout();
            s.connect(&replicate.output_2, &stdout_2.input);
        }))
    }
}

#[cfg(test)]
mod replicate_tests {
    use crate::{FlowBlocks, System};
    use protoflow_core::prelude::String;

    extern crate std;

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.replicate::<String>();
        });
    }
}
