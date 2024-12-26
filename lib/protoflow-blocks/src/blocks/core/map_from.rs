// This is free and unencumbered software released into the public domain.

use crate::System;
use protoflow_core::{Block, BlockResult, BlockRuntime, InputPort, Message, OutputPort};
use protoflow_derive::Block;
use simple_mermaid::mermaid;

/// A block to map a message from one type to another.
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/core/map_from.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/core/map_from.seq.mmd" framed)]
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
#[derive(Block, Clone)]
pub struct MapFrom<Input: Message, Output: Message + From<Input>> {
    /// The input message stream.
    #[input]
    pub input: InputPort<Input>,

    /// The output message stream.
    #[output]
    pub output: OutputPort<Output>,
}

impl<Input: Message, Output: Message + From<Input>> MapFrom<Input, Output> {
    pub fn new(input: InputPort<Input>, output: OutputPort<Output>) -> Self {
        Self::with_params(input, output)
    }
}

impl<Input: Message, Output: Message + From<Input>> MapFrom<Input, Output> {
    pub fn with_params(input: InputPort<Input>, output: OutputPort<Output>) -> Self {
        Self { input, output }
    }
}

impl<Input: Message + 'static, Output: Message + From<Input> + 'static> MapFrom<Input, Output> {
    pub fn with_system(system: &System) -> Self {
        use crate::SystemBuilding;
        Self::with_params(system.input(), system.output())
    }
}

impl<Input: Message, Output: Message + From<Input>> Block for MapFrom<Input, Output> {
    fn execute(&mut self, _: &dyn BlockRuntime) -> BlockResult {
        while let Some(input) = self.input.recv()? {
            let output: Output = From::from(input);
            self.output.send(&output)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::MapFrom;
    use crate::{System, SystemBuilding};

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.block(MapFrom::<u32, u64>::with_params(s.input(), s.output()));
        });
    }
}
