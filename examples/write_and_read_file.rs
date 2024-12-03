#!/usr/bin/env rust-script
//! This is free and unencumbered software released into the public domain.
//!
//! ```cargo
//! [dependencies]
//! protoflow = "0.4.3"
//! ```

use protoflow::{blocks::*, BlockResult};

fn main() -> BlockResult {
    System::run(|s| {
        let file_content = s.const_string("/home/user/documents\n/home/user/downloads\n/home/user/music\n/home/user/pictures\n/home/user/videos");
        let line_encoder = s.encode_lines();
        let content_path = s.const_string("content.txt");
        let write_file = s.write_file();
        s.connect(&file_content.output, &line_encoder.input);
        s.connect(&content_path.output, &write_file.path);
        s.connect(&line_encoder.output, &write_file.input);

        let file_path = s.const_string("content.txt");
        let reader = s.read_file();
        let stdout = s.write_stdout();
        s.connect(&file_path.output, &reader.path);
        s.connect(&reader.output, &stdout.input);
    })
}
