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
        let json_content = s.const_string(r#"{
            "Name": "Alice",
            "Age": 25,
            "Score": 90
        }"#);
        let line_encoder = s.encode_lines();
        let decoder = s.decode_json();
        let encoder = s.encode_json();
        let sanitized_path = s.const_string("sanitized.json");
        let write_file = s.write_file();
        s.connect(&json_content.output, &line_encoder.input);
        s.connect(&line_encoder.output, &decoder.input);
        s.connect(&decoder.output, &encoder.input);
        s.connect(&sanitized_path.output, &write_file.path);
        s.connect(&encoder.output, &write_file.input);
    })
}
