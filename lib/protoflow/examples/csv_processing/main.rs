// This is free and unencumbered software released into the public domain.

use protoflow::{blocks::*, BlockResult};

pub fn main() -> BlockResult {
    System::run(|s| {
        let csv = s.const_string("Name,Age,Score\nAlice,25,90\nBob,30,85\nCharlie,22,95");

        let encoder = s.encode_lines();
        s.connect(&csv.output, &encoder.input);

        let csv_decoder = s.decode_csv();
        s.connect(&encoder.output, &csv_decoder.input);

        let csv_encoder = s.encode_csv();
        s.connect(&csv_decoder.header, &csv_encoder.header);
        s.connect(&csv_decoder.rows, &csv_encoder.rows);

        let stdout = s.write_stdout();
        s.connect(&csv_encoder.output, &stdout.input);
    })
}
