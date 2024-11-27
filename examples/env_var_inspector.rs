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
        let name_param = s.const_string("PATH");
        let env_reader = s.read_env();
        let split_var = s.split_string(":");
        let line_encoder = s.encode_lines();
        let stdout = s.write_stdout();
        s.connect(&name_param.output, &env_reader.name);
        s.connect(&env_reader.output, &split_var.input);
        s.connect(&split_var.output, &line_encoder.input);
        s.connect(&line_encoder.output, &stdout.input);
    })
}
