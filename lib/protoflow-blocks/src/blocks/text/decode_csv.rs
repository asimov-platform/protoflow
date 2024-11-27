// This is free and unencumbered software released into the public domain.

extern crate std;

use crate::{
    prelude::{Bytes, ToString},
    StdioConfig, StdioError, StdioSystem, System,
};
use csv::ReaderBuilder;
use protoflow_core::{
    types::{value::Kind::*, ListValue as LV, Value},
    Block, BlockError, BlockResult, BlockRuntime, InputPort, OutputPort,
};
use protoflow_derive::Block;
use simple_mermaid::mermaid;
use std::io::Cursor;
/// A block that decodes CSV files from a byte stream into a header and rows represented as `prost_types::Value`.
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/text/decode_csv.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/text/decode_csv.seq.mmd" framed)]
///
/// # Examples
///
/// ## Using the block in a system
///
/// ```no_run
/// # use protoflow_blocks::*;
/// # fn main() {
/// System::build(|s| {
///     // TODO
/// });
/// # }
/// ```
///
/// ## Running the block via the CLI
///
/// ```console
/// $ protoflow execute DecodeCSV
/// ```
///
#[derive(Block, Clone)]
pub struct DecodeCsv {
    /// The input message bytes stream.
    #[input]
    pub input: InputPort<Bytes>,
    /// The csv header message.
    #[output]
    pub header: OutputPort<Value>,
    /// The csv rows message stream.
    #[output]
    pub rows: OutputPort<Value>,
    // TODO for the future to add a delimiter parameter.
}

impl DecodeCsv {
    pub fn new(
        input: InputPort<Bytes>,
        header: OutputPort<Value>,
        rows: OutputPort<Value>,
    ) -> Self {
        Self {
            input,
            header,
            rows,
        }
    }

    pub fn with_system(system: &System) -> Self {
        use crate::SystemBuilding;
        Self::new(system.input(), system.output(), system.output())
    }
}

impl Block for DecodeCsv {
    fn execute(&mut self, _runtime: &dyn BlockRuntime) -> BlockResult {
        while let Some(input) = self.input.recv()? {
            let cursor = Cursor::new(input);
            let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(cursor);

            let headers = rdr
                .headers()
                .map_err(|e| BlockError::Other(e.to_string()))?
                .iter()
                .map(|h| Value {
                    kind: Some(StringValue(h.to_string())),
                })
                .collect();

            let header_output = Value {
                kind: Some(ListValue(LV { values: headers })),
            };

            self.header.send(&header_output)?;

            for result in rdr.records() {
                let record = result.map_err(|e| BlockError::Other(e.to_string()))?;
                let row_values = record
                    .iter()
                    .map(|v| Value {
                        kind: Some(StringValue(v.to_string())),
                    })
                    .collect();

                let row_output = Value {
                    kind: Some(ListValue(LV { values: row_values })),
                };

                self.rows.send(&row_output)?;
            }
        }

        Ok(())
    }
}

#[cfg(feature = "std")]
impl StdioSystem for DecodeCsv {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        config.reject_any()?;

        Ok(System::build(|_s| todo!()))
    }
}

#[cfg(test)]
mod tests {

    use super::DecodeCsv;
    use crate::{System, SystemBuilding};

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.block(DecodeCsv::new(s.input(), s.output(), s.output()));
        });
    }
}
