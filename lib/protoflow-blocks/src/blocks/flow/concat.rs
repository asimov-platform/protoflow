// This is free and unencumbered software released into the public domain.
extern crate std;

use core::ops::DerefMut;

use crate::prelude::{Arc, Vec};
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
        runtime.wait_for(&self.output)?;

        let mut buffer1 = Vec::new();
        let mut buffer2 = Vec::new();

        let input1 = Arc::new(self.input_1.clone());
        let input2 = Arc::new(self.input_2.clone());

        let input1_clone = Arc::clone(&input1);
        let handle1 = std::thread::spawn(move || {
            while let Ok(Some(message)) = input1_clone.recv() {
                buffer1.push(message);
            }
            buffer1
        });

        let input2_clone = Arc::clone(&input2);
        let handle2 = std::thread::spawn(move || {
            while let Ok(Some(message)) = input2_clone.recv() {
                buffer2.push(message);
            }
            buffer2
        });

        let buffer1 = handle1.join().unwrap();
        let buffer2 = handle2.join().unwrap();

        let mut combined = buffer1;
        combined.extend(buffer2);

        for message in combined {
            self.output.send(&message)?;
        }
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

            let split = s.split();
            s.connect(&stdin.output, &split.input);

            let concat = s.block(Concat::new(s.input(), s.input(), s.output()));

            s.connect(&split.output_1, &concat.input_1);
            s.connect(&split.output_2, &concat.input_2);

            let stdout_1 = s.write_stdout();
            s.connect(&concat.output, &stdout_1.input);
        }))
    }
}

#[cfg(test)]
mod split_tests {
    use crate::{Concat, CoreBlocks, FlowBlocks, StdioSystem, SysBlocks, System};
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
        //use super::*;
        use protoflow_core::SystemBuilding;
        if let Err(e) = System::run(|s| {
            let stdin = s.read_stdin();

            let split = s.split();
            s.connect(&stdin.output, &split.input);

            let concat = s.block(Concat::new(s.input(), s.input(), s.output()));

            s.connect(&split.output_1, &concat.input_1);
            s.connect(&split.output_2, &concat.input_2);

            let stdout_1 = s.write_stdout();
            s.connect(&concat.output, &stdout_1.input);
        }) {
            error!("{}", e)
        }
    }
}
