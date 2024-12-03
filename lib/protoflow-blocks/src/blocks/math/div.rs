// This is free and unencumbered software released into the public domain.

extern crate std;

use crate::{StdioConfig, StdioError, StdioSystem, System};
use protoflow_core::{Block, BlockError, BlockResult, BlockRuntime, InputPort, OutputPort};
use protoflow_derive::Block;
use simple_mermaid::mermaid;

/// A block that divides numbers.
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/math/div.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/math/div.seq.mmd" framed)]
///
/// # Examples
///
/// ## Using the block in a system
///
/// ```rust
/// # use protoflow_blocks::*;
/// # fn main() {
/// System::build(|s| {
///     let config = StdioConfig {
///         encoding: Default::default(),
///         params: Default::default(),
///     };
///     let input = s.read_stdin();
///     let decode = s.decode_with::<f64>(config.encoding);
///     let div = s.div();
///     let encode = s.encode_with::<f64>(config.encoding);
///     let output = config.write_stdout(s);
///     s.connect(&input.output, &decode.input);
///     s.connect(&decode.output, &div.input);
///     s.connect(&div.output, &encode.input);
///     s.connect(&encode.output, &output.input);
/// });
/// # }
/// ```
///
/// ## Running the block via the CLI
///
/// ```console
/// $ protoflow execute Div
/// ```
///
#[derive(Block, Clone)]
pub struct Div {
    /// The input number stream.
    #[input]
    pub input: InputPort<f64>,
    /// The output stream of running totals.
    #[output]
    pub output: OutputPort<f64>,
}

impl Div {
    pub fn new(input: InputPort<f64>, output: OutputPort<f64>) -> Self {
        Self { input, output }
    }

    pub fn with_system(system: &System) -> Self {
        use crate::SystemBuilding;
        Self::new(system.input(), system.output())
    }
}

impl Block for Div {
    fn execute(&mut self, _runtime: &dyn BlockRuntime) -> BlockResult {
        let mut result = None;
        while let Some(input) = self.input.recv()? {
            let res = match result {
                None => input,
                Some(current_result) => {
                    if input == 0.0 {
                        return Err(BlockError::Other("Division by zero".into()));
                    }

                    current_result / input
                }
            };

            result = Some(res);
            self.output.send(&res)?;
        }

        Ok(())
    }
}

#[cfg(feature = "std")]
impl StdioSystem for Div {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        use crate::{IoBlocks, MathBlocks, SysBlocks, SystemBuilding};

        config.reject_any()?;

        Ok(System::build(|s| {
            let input = s.read_stdin();
            let decode = s.decode_with::<f64>(config.encoding);
            let div = s.div();
            let encode = s.encode_with::<f64>(config.encoding);
            let output = config.write_stdout(s);
            s.connect(&input.output, &decode.input);
            s.connect(&decode.output, &div.input);
            s.connect(&div.output, &encode.input);
            s.connect(&encode.output, &output.input);
        }))
    }
}

#[cfg(test)]
mod tests {

    use super::Div;
    use crate::{System, SystemBuilding};

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.block(Div::new(s.input(), s.output()));
        });
    }
}
