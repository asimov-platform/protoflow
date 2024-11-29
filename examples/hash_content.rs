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
        let content = s.const_string("Hello, World!");
        let line_encoder = s.encode_lines();
        let hash_content = s.hash_sha2();
        let stdout = s.write_stdout();
        s.connect(&content.output, &line_encoder.input);
        s.connect(&line_encoder.output, &hash_content.input);
        s.connect(&hash_content.hash, &stdout.input);
    })
}
