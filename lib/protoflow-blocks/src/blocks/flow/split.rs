// This is free and unencumbered software released into the public domain.

use crate::{FlowBlocks, StdioConfig, StdioError, StdioSystem, SysBlocks, System};
use protoflow_core::{
    types::Any, Block, BlockResult, BlockRuntime, InputPort, Message, OutputPort,
};
use protoflow_derive::Block;
use simple_mermaid::mermaid;

/// Divides a single input message stream into multiple output streams using a round-robin approach.
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/flow/split.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/flow/split.seq.mmd" framed)]
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
///     let split = s.split();
///     s.connect(&stdin.output, &split.input);
///     let stdout_1 = s.write_stdout();
///     s.connect(&split.output_1, &stdout_1.input);
///     let stdout_2 = s.write_stdout();
///     s.connect(&split.output_2, &stdout_2.input);
/// });
/// # }
/// ```
///
/// ## Running the block via the CLI
///
/// ```console
/// $ protoflow execute Split
/// ```
///
#[derive(Block, Clone)]
pub struct Split<T: Message = Any> {
    /// The input message stream.
    #[input]
    pub input: InputPort<T>,
    /// The output message stream
    #[output]
    pub output_1: OutputPort<T>,
    /// The output message stream
    #[output]
    pub output_2: OutputPort<T>,
    /// The internal state for keeping total number of messages
    #[state]
    pub message_count: u128,
}

impl<T: Message> Split<T> {
    pub fn new(input: InputPort<T>, output_1: OutputPort<T>, output_2: OutputPort<T>) -> Self {
        Self {
            input,
            output_1,
            output_2,
            message_count: 0,
        }
    }
}
impl<T: Message + 'static> Split<T> {
    pub fn with_system(system: &System) -> Self {
        use crate::SystemBuilding;
        Self::new(system.input(), system.output(), system.output())
    }
}

impl<T: Message> Block for Split<T> {
    fn execute(&mut self, runtime: &dyn BlockRuntime) -> BlockResult {
        runtime.wait_for(&self.output_1)?;
        runtime.wait_for(&self.output_2)?;
        while let Some(message) = self.input.recv()? {
            match self.message_count % 2 {
                0 => self.output_1.send(&message)?,
                1 => self.output_2.send(&message)?,
                _ => {}
            }
            self.message_count += 1;
        }
        Ok(())
    }
}

#[cfg(feature = "std")]
impl<T: Message> StdioSystem for Split<T> {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        use crate::SystemBuilding;
        config.reject_any()?;

        Ok(System::build(|s| {
            let stdin = s.read_stdin();

            let split = s.split();
            s.connect(&stdin.output, &split.input);

            let stdout_1 = s.write_stdout();
            s.connect(&split.output_1, &stdout_1.input);

            let stdout_2 = s.write_stdout();
            s.connect(&split.output_2, &stdout_2.input);
        }))
    }
}

#[cfg(test)]
mod split_tests {
    use crate::{CoreBlocks, FlowBlocks, SysBlocks, System};
    use protoflow_core::prelude::String;
    use tracing::error;
    extern crate std;

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.split::<String>();
        });
    }

    #[test]
    #[ignore = "requires stdin"]
    fn run_split_stdout_and_file() {
        use super::*;
        use protoflow_core::SystemBuilding;
        if let Err(e) = System::run(|s| {
            let stdin = s.read_stdin();
            let split = s.split();
            s.connect(&stdin.output, &split.input);

            let stdout_1 = s.write_stdout();
            s.connect(&split.output_1, &stdout_1.input);

            let file = s.const_string("text.txt");
            let write_file = s.write_file().with_flags(crate::WriteFlags {
                create: true,
                append: true,
            });
            s.connect(&file.output, &write_file.path);
            s.connect(&split.output_2, &write_file.input);
        }) {
            error!("{}", e)
        }
    }

    #[test]
    #[ignore = "requires stdin"]
    fn run_split_to_stdout() {
        use protoflow_core::SystemBuilding;
        if let Err(e) = System::run(|s| {
            let stdin = s.read_stdin();

            let split = s.split();
            s.connect(&stdin.output, &split.input);

            let stdout_1 = s.write_stdout();
            s.connect(&split.output_1, &stdout_1.input);

            let stdout_2 = s.write_stdout();
            s.connect(&split.output_2, &stdout_2.input);
        }) {
            error!("{}", e)
        }
    }
}
