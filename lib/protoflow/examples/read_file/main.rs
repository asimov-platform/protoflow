// This is free and unencumbered software released into the public domain.

use protoflow::{blocks::*, BlockResult};

pub fn main() -> BlockResult {
    System::run(|s| {
        let initial_content = s.const_string("This is line 1\nThis is line 2\nThis is line 3");
        let encoder = s.encode_lines();
        s.connect(&initial_content.output, &encoder.input);

        let write_file = s.write_file();
        let write_path = s.const_string("self_contained_file.txt");
        s.connect(&encoder.output, &write_file.input);
        s.connect(&write_path.output, &write_file.path);

        let read_path = s.const_string("self_contained_file.txt");
        let read_file = s.read_file();
        s.connect(&read_path.output, &read_file.path);

        let stdout = s.write_stdout();
        s.connect(&read_file.output, &stdout.input);
    })
}
