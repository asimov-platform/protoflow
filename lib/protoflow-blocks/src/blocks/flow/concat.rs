// This is free and unencumbered software released into the public domain.
extern crate std;

use crate::prelude::{Arc, Vec};
use crate::{FlowBlocks, StdioConfig, StdioError, StdioSystem, SysBlocks, System};
use protoflow_core::{
    types::Any, Block, BlockResult, BlockRuntime, InputPort, Message, OutputPort,
};
use protoflow_core::{BlockError, PortError};
use protoflow_derive::Block;
use simple_mermaid::mermaid;

/// Combines multiple input streams into a single output stream in sequence.
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/flow/concat.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/flow/concat.seq.mmd" framed)]
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
///     let concat = s.block(Concat::new(s.input(), s.input(), s.output()));
///     s.connect(&replicate.output_1, &concat.input_1);
///     s.connect(&replicate.output_2, &concat.input_2);
///
///     let stdout_1 = s.write_stdout();
///     s.connect(&concat.output, &stdout_1.input);
/// });
/// # }
/// ```
///
/// ## Running the block via the CLI
///
/// ```console
/// $ protoflow execute Concat
/// ```
///
#[derive(Block, Clone)]
pub struct Concat<T: Message = Any> {
    /// The input message stream.
    #[input]
    pub input_1: InputPort<T>,
    #[output]
    pub input_2: InputPort<T>,
    #[output]
    pub output: OutputPort<T>,
}

impl<T: Message> Concat<T> {
    pub fn new(input_1: InputPort<T>, input_2: InputPort<T>, output: OutputPort<T>) -> Self {
        Self {
            input_1,
            input_2,
            output,
        }
    }
}
impl<T: Message + 'static> Concat<T> {
    pub fn with_system(system: &System) -> Self {
        use crate::SystemBuilding;
        Self::new(system.input(), system.input(), system.output())
    }
}

impl<T: Message + Send + 'static> Block for Concat<T> {
    fn execute(&mut self, runtime: &dyn BlockRuntime) -> BlockResult {
        // Ensure the output channel is ready
        runtime.wait_for(&self.output)?;

        let input1 = Arc::new(self.input_1.clone());
        let input2 = Arc::new(self.input_2.clone());

        // Helper function to buffer messages from an input
        fn buffer_input<T: Message>(
            input: Arc<InputPort<T>>,
            input_name: &str,
        ) -> Result<Vec<T>, PortError> {
            let mut buffer = Vec::new();
            while let Ok(Some(message)) = input.recv() {
                buffer.push(message);
            }
            tracing::info!("{} processed {} messages", input_name, buffer.len());
            Ok(buffer)
        }

        // Spawn threads to process and buffer messages from both inputs
        let handle1 = std::thread::spawn({
            let input1 = Arc::clone(&input1);
            move || buffer_input(input1, "input1")
        });

        let handle2 = std::thread::spawn({
            let input2 = Arc::clone(&input2);
            move || buffer_input(input2, "input2")
        });

        // Collect and handle thread results
        let buffer1 = match handle1.join() {
            Ok(result) => result.map_err(|e| {
                tracing::error!("Error processing input1: {:?}", e);
                BlockError::Other("Failed to process input1".into())
            })?,
            Err(_) => {
                tracing::error!("Thread for input1 panicked");
                return Err(BlockError::Other("Thread for input1 panicked".into()));
            }
        };

        let buffer2 = match handle2.join() {
            Ok(result) => result.map_err(|e| {
                tracing::error!("Error processing input2: {:?}", e);
                BlockError::Other("Failed to process input2".into())
            })?,
            Err(_) => {
                tracing::error!("Thread for input2 panicked");
                return Err(BlockError::Other("Thread for input2 panicked".into()));
            }
        };

        // Concatenate and send messages to the output sequentially
        tracing::info!(
            "Concatenating {} messages from input1 with {} messages from input2",
            buffer1.len(),
            buffer2.len()
        );

        for message in buffer1.iter().chain(buffer2.iter()) {
            if let Err(err) = self.output.send(message) {
                tracing::error!("Failed to send message: {:?}", err);
                return Err(err.into());
            }
        }

        tracing::info!("All messages successfully sent to the output.");
        Ok(())
    }
}

#[cfg(feature = "std")]
impl<T: Message> StdioSystem for Concat<T> {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        use crate::SystemBuilding;
        config.reject_any()?;

        Ok(System::build(|s| {
            let stdin = s.read_stdin();

            let replicate = s.replicate();
            s.connect(&stdin.output, &replicate.input);

            let concat = s.block(Concat::new(s.input(), s.input(), s.output()));

            s.connect(&replicate.output_1, &concat.input_1);
            s.connect(&replicate.output_2, &concat.input_2);

            let stdout_1 = s.write_stdout();
            s.connect(&concat.output, &stdout_1.input);
        }))
    }
}

#[cfg(test)]
mod concat_tests {
    use crate::{FlowBlocks, System};
    use protoflow_core::prelude::String;

    extern crate std;

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.concat::<String>();
        });
    }
}
