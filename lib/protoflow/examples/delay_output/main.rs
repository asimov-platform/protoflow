// This is free and unencumbered software released into the public domain.

use protoflow::{blocks::*, BlockResult};

pub fn main() -> BlockResult {
    System::run(|s| {
        let input_string = s.const_string("Line 1\nLine 2\nLine 3\nLine 4");

        let split_string = s.split_string("\n");
        s.connect(&input_string.output, &split_string.input);

        let delay = s.delay();
        s.connect(&split_string.output, &delay.input);

        let encoder = s.encode_lines();
        s.connect(&delay.output, &encoder.input);

        let stdout = s.write_stdout();
        s.connect(&encoder.output, &stdout.input);
    })
}
