// This is free and unencumbered software released into the public domain.

use crate::{StdioConfig, StdioError, StdioSystem, System};
use protoflow_core::{
    prelude::Vec, types::Any, Block, BlockResult, BlockRuntime, InputPort, Message, OutputPort,
};
use protoflow_derive::Block;

/// A block that simply stores all messages it receives.
///
/// # Block Diagram

///
/// # Sequence Diagram

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
///     let sort = s.sort();
///     s.connect(&stdin.output, &sort.input);
/// });
/// # }
/// ```
///
/// ## Running the block via the CLI
///
/// ```console
/// $ protoflow execute Sort
/// ```
///
#[derive(Block, Clone)]
pub struct Sort<T: Message = Any> {
    /// The input message stream.
    #[input]
    pub input: InputPort<T>,

    #[input]
    pub stop: InputPort<T>,

    /// The output message stream.
    #[output]
    pub output: OutputPort<T>,

    /// The internal state storing the messages received.
    #[state]
    messages: Vec<T>,
}

impl<T: Message> Sort<T> {
    pub fn new(input: InputPort<T>, stop: InputPort<T>, output: OutputPort<T>) -> Self {
        Self {
            input,
            stop,
            output,
            messages: Vec::new(),
        }
    }

    pub fn messages(&self) -> &Vec<T> {
        &self.messages
    }
}

impl<T: Message + 'static> Sort<T> {
    pub fn with_system(system: &System) -> Self {
        use crate::SystemBuilding;
        Self::new(system.input(), system.input(), system.output())
    }
}

impl<T: Message> Block for Sort<T> {
    fn execute(&mut self, _runtime: &dyn BlockRuntime) -> BlockResult {
        // if let Some(x) = self.stop.recv()? {
        //     while let Some(message) = self.input.recv()? {
        //         if message == x {
        //             self.messages.sort_by(|x, y| x.partial_cmp(y).unwrap());
        //             for x in self.messages.iter() {
        //                 self.output.send(x)?;
        //             }
        //             self.messages.clear();
        //         } else {
        //             self.messages.push(message);
        //         }
        //     }
        // }
        Ok(())
    }
}

#[cfg(feature = "std")]
impl<T: Message> StdioSystem for Sort<T> {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        use crate::SystemBuilding;

        config.reject_any()?;

        Ok(System::build(|s| {
            let stdin = config.read_stdin(s);
            let sort = s.block(Sort::new(s.input(), s.input(), s.output()));
            s.connect(&stdin.output, &sort.input);
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::Sort;
    use crate::{System, SystemBuilding};

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.block(Sort::<i32>::new(s.input(), s.input(), s.output()));
        });
    }
}

#[cfg(test)]
mod split_tests {
    use bytes::Bytes;
    use tracing::error;

    use crate::{Const, SysBlocks};

    #[test]
    #[ignore = "requires stdin"]
    fn run_split_stdout_and_file() {
        use super::*;
        use protoflow_core::SystemBuilding;
        if let Err(e) = System::run(|s| {
            let stdin = s.read_stdin();
            let split = s.block(Sort::new(s.input(), s.input(), s.output()));
            s.connect(&stdin.output, &split.input);

            let constant = s.block(Const::<Bytes>::with_params(s.output(), Bytes::from("\n")));
            s.connect(&constant.output, &split.stop);

            let stdout_1 = s.write_stdout();
            s.connect(&split.output, &stdout_1.input);
        }) {
            error!("{}", e)
        }
    }
}
