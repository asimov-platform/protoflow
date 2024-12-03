use std::io::Result;
fn main() -> Result<()> {
    prost_build::Config::default()
        .out_dir("src/")
        .compile_protos(&["src/transport_event.proto"], &["src/"])
}
