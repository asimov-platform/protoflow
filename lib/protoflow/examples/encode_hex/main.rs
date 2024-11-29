// This is free and unencumbered software released into the public domain.

use protoflow::{blocks::*, BlockResult};

pub fn main() -> BlockResult {
    System::run(|s| {
        let input_string = s.const_string("The quick brown fox jumps over the lazy dog");

        let encoder = s.encode_lines();
        s.connect(&input_string.output, &encoder.input);

        let hex_encoder = s.encode_hex();
        s.connect(&encoder.output, &hex_encoder.input);

        let stdout = s.write_stdout();
        s.connect(&hex_encoder.output, &stdout.input);
    })
}
