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
        // The path to a directory
        let dir_path = s.const_string("/");
        let dir_reader = s.read_dir();
        let concat_paths = s.concat_strings_by("\n");
        let line_encoder = s.encode_lines();
        let write_path = s.const_string("log_directories.txt");
        let write_file = s.write_file();
        s.connect(&dir_path.output, &dir_reader.path);
        s.connect(&dir_reader.output, &concat_paths.input);
        s.connect(&concat_paths.output, &line_encoder.input);
        s.connect(&write_path.output, &write_file.path);
        s.connect(&line_encoder.output, &write_file.input);
    })
}
