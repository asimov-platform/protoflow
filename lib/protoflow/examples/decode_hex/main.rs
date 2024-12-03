// This is free and unencumbered software released into the public domain.

use protoflow::{blocks::*, BlockResult};

pub fn main() -> BlockResult {
    System::run(|s| {
        let input_string = s.const_string("54686520717569636b2062726f776e20666f78206a756d7073206f76657220746865206c617a7920646f670a");

        let encoder = s.encode_lines();
        s.connect(&input_string.output, &encoder.input);

        let hex_decoder = s.decode_hex();
        s.connect(&encoder.output, &hex_decoder.input);

        let stdout = s.write_stdout();
        s.connect(&hex_decoder.output, &stdout.input);
    })
}
