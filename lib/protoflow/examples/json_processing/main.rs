// This is free and unencumbered software released into the public domain.

use protoflow::{blocks::*, BlockResult};

pub fn main() -> BlockResult {
    System::run(|s| {
        let json = s.const_string(r#"{"Name": "Alice","Age": 25,"Score": 90}"#);

        let line_encoder = s.encode_lines();
        s.connect(&json.output, &line_encoder.input);

        let decoder = s.decode_json();
        s.connect(&line_encoder.output, &decoder.input);

        let encoder = s.encode_json();
        s.connect(&decoder.output, &encoder.input);

        let stdout = s.write_stdout();
        s.connect(&encoder.output, &stdout.input);
    })
}
