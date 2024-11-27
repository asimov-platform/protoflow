// This is free and unencumbered software released into the public domain.

extern crate std;

use crate::{
    prelude::{vec, String, ToString},
    StdioConfig, StdioError, StdioSystem, System,
};
use protoflow_core::{Block, BlockResult, BlockRuntime, InputPort, OutputPort};
use protoflow_derive::Block;
use simple_mermaid::mermaid;

/// A block that splits string.
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/text/split_string.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/text/split_string.seq.mmd" framed)]
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
///     let delimiter = " ";
///     let stdin = s.read_stdin();
///     let line_decoder = s.decode_with(config.encoding);
///     let split_string = s.split_string(delimiter);
///     let line_encoder = s.encode_with(config.encoding);
///     let stdout = config.write_stdout(s);
///     s.connect(&stdin.output, &line_decoder.input);
///     s.connect(&line_decoder.output, &split_string.input);
///     s.connect(&split_string.output, &line_encoder.input);
///     s.connect(&line_encoder.output, &stdout.input);
/// });
/// # }
/// ```
///
/// ## Running the block via the CLI
///
/// ```console
/// $ protoflow execute SplitString delimiter=" "
/// ```
///
#[derive(Block, Clone)]
pub struct SplitString {
    /// The input message string stream.
    #[input]
    pub input: InputPort<String>,
    /// The output message string stream.
    #[output]
    pub output: OutputPort<String>,
    /// A parameter to split the input string
    #[parameter]
    pub delimiter: String,
}

impl SplitString {
    pub fn new(input: InputPort<String>, output: OutputPort<String>) -> Self {
        Self::with_params(input, output, None)
    }

    pub fn with_system(system: &System, delimiter: Option<String>) -> Self {
        use crate::SystemBuilding;
        Self::with_params(system.input(), system.output(), delimiter)
    }

    pub fn with_params(
        input: InputPort<String>,
        output: OutputPort<String>,
        delimiter: Option<String>,
    ) -> Self {
        Self {
            input,
            output,
            delimiter: delimiter.unwrap_or_default(),
        }
    }
}

impl Block for SplitString {
    fn execute(&mut self, runtime: &dyn BlockRuntime) -> BlockResult {
        runtime.wait_for(&self.input)?;

        while let Some(input) = self.input.recv()? {
            for output in input.split(&self.delimiter) {
                self.output.send(&output.to_string())?;
            }
        }

        Ok(())
    }
}

#[cfg(feature = "std")]
impl StdioSystem for SplitString {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        use crate::{IoBlocks, SysBlocks, SystemBuilding, TextBlocks};

        config.allow_only(vec!["delimiter"])?;
        let delimiter = config.get_string("delimiter")?;

        Ok(System::build(|s| {
            let stdin = s.read_stdin();
            let line_decoder = s.decode_with(config.encoding);
            let split_string = s.split_string(&delimiter);
            let line_encoder = s.encode_with(config.encoding);
            let stdout = config.write_stdout(s);
            s.connect(&stdin.output, &line_decoder.input);
            s.connect(&line_decoder.output, &split_string.input);
            s.connect(&split_string.output, &line_encoder.input);
            s.connect(&line_encoder.output, &stdout.input);
        }))
    }
}

#[cfg(test)]
mod tests {

    use super::SplitString;
    use crate::{System, SystemBuilding};

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.block(SplitString::new(s.input(), s.output()));
        });
    }
}
