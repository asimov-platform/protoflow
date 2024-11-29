// This is free and unencumbered software released into the public domain.

use protoflow::{blocks::*, BlockResult};

pub fn main() -> BlockResult {
    System::run(|s| {
        let env_var_name = s.const_string("PATH");
        let read_env = s.read_env();
        s.connect(&env_var_name.output, &read_env.name);

        let split_env = s.split_string(":");
        s.connect(&read_env.output, &split_env.input);

        let transform_paths = s.concat_strings_by("\n");
        s.connect(&split_env.output, &transform_paths.input);

        let line_encoder = s.encode_lines();
        s.connect(&transform_paths.output, &line_encoder.input);

        let write_stdout = s.write_stdout();
        s.connect(&line_encoder.output, &write_stdout.input);
    })
}
