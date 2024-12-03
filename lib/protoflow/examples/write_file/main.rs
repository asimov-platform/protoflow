// This is free and unencumbered software released into the public domain.

use protoflow::{blocks::*, BlockResult};

pub fn main() -> BlockResult {
    System::run(|s| {
        let stdin = s.read_stdin();
        let file_path = s.const_string("output.txt");
        let write_file = s.write_file();
        s.connect(&stdin.output, &write_file.input);
        s.connect(&file_path.output, &write_file.path);
    })
}
