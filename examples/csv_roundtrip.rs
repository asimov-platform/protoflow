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
        let csv_content = s.const_string("Name,Age,Score\nAlice,25,90\nBob,30,85\nCharlie,22,95");
        let line_encoder = s.encode_lines();
        let decoder = s.decode_csv();
        let encoder = s.encode_csv();
        let output_path = s.const_string("output.csv");
        let write_file = s.write_file();
        s.connect(&csv_content.output, &line_encoder.input);
        s.connect(&line_encoder.output, &decoder.input);
        s.connect(&decoder.header, &encoder.header);
        s.connect(&decoder.rows, &encoder.rows);
        s.connect(&output_path.output, &write_file.path);
        s.connect(&encoder.output, &write_file.input);
    })
}
