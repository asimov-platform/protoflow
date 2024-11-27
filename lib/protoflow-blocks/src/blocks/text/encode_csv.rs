// This is free and unencumbered software released into the public domain.

extern crate std;

use crate::{
    prelude::{Bytes, String, ToString, Vec},
    StdioConfig, StdioError, StdioSystem, System,
};
use csv::WriterBuilder;
use protoflow_core::{
    types::{value::Kind::*, Value},
    Block, BlockError, BlockResult, BlockRuntime, InputPort, OutputPort,
};
use protoflow_derive::Block;
use simple_mermaid::mermaid;
use std::io::Cursor;
/// A block that encodes CSV files by converting a header and rows, provided as `prost_types::Value` streams, into a byte stream.
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/text/encode_csv.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/text/encode_csv.seq.mmd" framed)]
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
/// $ protoflow execute EncodeCSV
/// ```
///
#[derive(Block, Clone)]
pub struct EncodeCsv {
    /// The header message proto_types::Value stream.
    #[input]
    pub header: InputPort<Value>,
    /// The rows message proto_types::Value stream.
    #[input]
    pub rows: InputPort<Value>,
    /// The output message stream.
    #[output]
    pub output: OutputPort<Bytes>,
    // TODO for the future to add a delimiter parameter.
}

impl EncodeCsv {
    pub fn new(
        header: InputPort<Value>,
        rows: InputPort<Value>,
        output: OutputPort<Bytes>,
    ) -> Self {
        Self {
            header,
            rows,
            output,
        }
    }

    pub fn with_system(system: &System) -> Self {
        use crate::SystemBuilding;
        Self::new(system.input(), system.input(), system.output())
    }
}

impl Block for EncodeCsv {
    fn execute(&mut self, _runtime: &dyn BlockRuntime) -> BlockResult {
        if let Some(header) = self.header.recv()? {
            self.output.send(&encode_value_to_csv(&header)?)?;
        }

        while let Some(row) = self.rows.recv()? {
            self.output.send(&encode_value_to_csv(&row)?)?;
        }

        Ok(())
    }
}

fn encode_value_to_csv(value: &Value) -> Result<Bytes, BlockError> {
    let mut buffer = Cursor::new(Vec::new());

    {
        let mut wtr = WriterBuilder::new()
            .has_headers(false)
            .from_writer(&mut buffer);

        if let Some(ListValue(lv)) = &value.kind {
            let row: Vec<String> = lv
                .values
                .iter()
                .filter_map(|v| {
                    if let Some(StringValue(s)) = &v.kind {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
                .collect();

            wtr.write_record(row)
                .map_err(|e| BlockError::Other(e.to_string()))?;
            wtr.flush().map_err(|e| BlockError::Other(e.to_string()))?;
        }
    }

    Ok(Bytes::from(buffer.into_inner()))
}

#[cfg(feature = "std")]
impl StdioSystem for EncodeCsv {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        config.reject_any()?;

        Ok(System::build(|_s| todo!()))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::prelude::vec;
    use crate::{System, SystemBuilding};
    use protoflow_core::types::{value::Kind, Value};

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.block(EncodeCsv::new(s.input(), s.input(), s.output()));
        });
    }

    #[test]
    fn test_encode_value_to_csv_valid_header() {
        let value = Value {
            kind: Some(ListValue(protoflow_core::types::ListValue {
                values: vec![
                    Value {
                        kind: Some(StringValue("col1".to_string())),
                    },
                    Value {
                        kind: Some(StringValue("col2".to_string())),
                    },
                    Value {
                        kind: Some(StringValue("col3".to_string())),
                    },
                ],
            })),
        };

        let bytes = encode_value_to_csv(&value).expect("Encoding should succeed");
        assert_eq!(
            String::from_utf8(bytes.to_vec()).unwrap(),
            "col1,col2,col3\n"
        );
    }

    #[test]
    fn test_encode_value_to_csv_empty_row() {
        let value = Value {
            kind: Some(ListValue(protoflow_core::types::ListValue {
                values: vec![],
            })),
        };

        let bytes = encode_value_to_csv(&value).expect("Encoding should succeed");
        assert_eq!(String::from_utf8(bytes.to_vec()).unwrap(), "\"\"\n");
    }

    #[test]
    fn test_encode_value_to_csv_ignores_non_string_values() {
        let value = Value {
            kind: Some(ListValue(protoflow_core::types::ListValue {
                values: vec![
                    Value {
                        kind: Some(StringValue("valid".to_string())),
                    },
                    Value { kind: None }, // Invalid (None)
                    Value {
                        kind: Some(Kind::NumberValue(42.0)),
                    }, // Unsupported type
                ],
            })),
        };

        let bytes = encode_value_to_csv(&value).expect("Encoding should succeed");
        assert_eq!(String::from_utf8(bytes.to_vec()).unwrap(), "valid\n");
    }

    #[test]
    fn test_encode_value_to_csv_invalid_value() {
        let value = Value {
            kind: None, // No kind in value
        };

        let bytes = encode_value_to_csv(&value).expect("Encoding should succeed");
        assert_eq!(String::from_utf8(bytes.to_vec()).unwrap(), "");
    }
}
