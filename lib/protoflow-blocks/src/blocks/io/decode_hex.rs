// This is free and unencumbered software released into the public domain.

use crate::{
    prelude::{format, Bytes, Vec},
    IoBlocks, StdioConfig, StdioError, StdioSystem, System,
};
use protoflow_core::{error, Block, BlockError, BlockResult, BlockRuntime, InputPort, OutputPort};
use protoflow_derive::Block;
use simple_mermaid::mermaid;

/// A block that decodes a hexadecimal byte stream to byte.
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/io/decode_hex.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/io/decode_hex.seq.mmd" framed)]
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
///     let hex_decoder = s.decode_hex();
///     let stdout = s.write_stdout();
///     s.connect(&stdin.output, &hex_decoder.input);
///     s.connect(&hex_decoder.output, &stdout.input);
/// });
/// # }
/// ```
///
/// ## Running the block via the CLI
///
/// ```console
/// $ protoflow execute DecodeHex
/// ```
///
#[derive(Block, Clone)]
pub struct DecodeHex {
    /// The input text stream.
    #[input]
    pub input: InputPort<Bytes>,

    /// The output byte stream.
    #[output]
    pub output: OutputPort<Bytes>,
}

impl DecodeHex {
    pub fn new(input: InputPort<Bytes>, output: OutputPort<Bytes>) -> Self {
        Self { input, output }
    }

    pub fn with_system(system: &System) -> Self {
        use crate::SystemBuilding;
        Self::new(system.input(), system.output())
    }
}

impl Block for DecodeHex {
    fn execute(&mut self, runtime: &dyn BlockRuntime) -> BlockResult {
        runtime.wait_for(&self.input)?;

        while let Some(message) = self.input.recv()? {
            let decoded = hex_to_bytes(&message)?;
            self.output.send(&decoded)?;
        }

        Ok(())
    }
}

fn hex_to_bytes(hex_message: &Bytes) -> Result<Bytes, BlockError> {
    let mut decoded = Vec::with_capacity(hex_message.len() / 2);

    for chunk in hex_message.chunks_exact(2) {
        let high = chunk[0];
        let low = chunk[1];
        decoded.push((hex_value(high)? << 4) | hex_value(low)?);
    }

    Ok(Bytes::from(decoded))
}

#[inline(always)]
fn hex_value(byte: u8) -> Result<u8, BlockError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => {
            let err = format!("Invalid hex character: '{}' (0x{:02X})", byte as char, byte);
            error!(target: "DecodeHex:hex_value", err);
            Err(BlockError::Other(err))
        }
    }
}

#[cfg(feature = "std")]
impl StdioSystem for DecodeHex {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        use crate::SystemBuilding;

        config.reject_any()?;

        Ok(System::build(|s| {
            let stdin = config.read_stdin(s);
            let hex_decoder = s.decode_hex();
            let stdout = config.write_stdout(s);
            s.connect(&stdin.output, &hex_decoder.input);
            s.connect(&hex_decoder.output, &stdout.input);
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::DecodeHex;
    use crate::{SysBlocks, System, SystemBuilding};

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.block(DecodeHex::new(s.input(), s.output()));
        });
    }

    #[test]
    #[ignore]
    fn test_encode_decode_hex() {
        use super::*;

        let _ = System::run(|s| {
            let stdin = s.read_stdin();
            let hex_encoder = s.encode_hex();
            s.connect(&stdin.output, &hex_encoder.input);

            let hex_decoder = s.decode_hex();
            s.connect(&hex_encoder.output, &hex_decoder.input);

            let stdout = s.write_stdout();
            s.connect(&hex_decoder.output, &stdout.input);
        });
    }
}
