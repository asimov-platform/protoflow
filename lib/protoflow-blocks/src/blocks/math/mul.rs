// This is free and unencumbered software released into the public domain.

extern crate std;

use crate::{StdioConfig, StdioError, StdioSystem, System};
use protoflow_core::{Block, BlockResult, BlockRuntime, InputPort, OutputPort};
use protoflow_derive::Block;
use simple_mermaid::mermaid;

/// A block that multiplies numbers.
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/math/mul.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/math/mul.seq.mmd" framed)]
///
/// # Examples
///
/// ## Using the block in a system
///
/// ```no_run
/// # use protoflow_blocks::*;
/// # fn main() {
/// System::build(|s| {
///     let config = StdioConfig {
///         encoding: Default::default(),
///         params: Default::default(),
///     };
///     let input = s.read_stdin();
///     let decode = s.decode_with::<f64>(config.encoding);
///     let mul = s.mul();
///     let encode = s.encode_with::<f64>(config.encoding);
///     let output = config.write_stdout(s);
///     s.connect(&input.output, &decode.input);
///     s.connect(&decode.output, &mul.input);
///     s.connect(&mul.output, &encode.input);
///     s.connect(&encode.output, &output.input);
/// });
/// # }
/// ```
///
/// ## Running the block via the CLI
///
/// ```console
/// $ protoflow execute Mul
/// ```
///
#[derive(Block, Clone)]
pub struct Mul {
    /// The input number stream.
    #[input]
    pub input: InputPort<f64>,
    /// The output stream of running totals.
    #[output]
    pub output: OutputPort<f64>,
}

impl Mul {
    pub fn new(input: InputPort<f64>, output: OutputPort<f64>) -> Self {
        Self { input, output }
    }

    pub fn with_system(system: &System) -> Self {
        use crate::SystemBuilding;
        Self::new(system.input(), system.output())
    }
}

impl Block for Mul {
    fn execute(&mut self, _runtime: &dyn BlockRuntime) -> BlockResult {
        let mut product = None;
        while let Some(input) = self.input.recv()? {
            let res = match product {
                None => input,
                Some(current_product) => current_product * input,
            };

            product = Some(res);
            self.output.send(&res)?;
        }

        Ok(())
    }
}

#[cfg(feature = "std")]
impl StdioSystem for Mul {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        use crate::{IoBlocks, MathBlocks, SysBlocks, SystemBuilding};

        config.reject_any()?;

        Ok(System::build(|s| {
            let input = s.read_stdin();
            let decode = s.decode_with::<f64>(config.encoding);
            let mul = s.mul();
            let encode = s.encode_with::<f64>(config.encoding);
            let output = config.write_stdout(s);
            s.connect(&input.output, &decode.input);
            s.connect(&decode.output, &mul.input);
            s.connect(&mul.output, &encode.input);
            s.connect(&encode.output, &output.input);
        }))
    }
}

#[cfg(test)]
mod tests {

    use super::Mul;
    use crate::{System, SystemBuilding};

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.block(Mul::new(s.input(), s.output()));
        });
    }
}
