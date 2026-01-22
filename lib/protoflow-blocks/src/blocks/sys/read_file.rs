// This is free and unencumbered software released into the public domain.

extern crate std;

use crate::{
    prelude::{vec, Bytes, String, Vec},
    StdioConfig, StdioError, StdioSystem, System,
};
use protoflow_core::{Block, BlockResult, BlockRuntime, InputPort, OutputPort};
use protoflow_derive::Block;
use simple_mermaid::mermaid;

/// A block that reads bytes from the contents of a file.
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/sys/read_file.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/sys/read_file.seq.mmd" framed)]
///
/// # Examples
///
/// ## Using the block in a system
///
/// ```rust
/// # use protoflow_blocks::*;
/// # fn main() {
/// System::build(|s| {
///     // TODO
/// });
/// # }
/// ```
///
/// ## Running the block via the CLI
///
/// ```console
/// $ protoflow execute ReadFile path=/tmp/file.txt
/// ```
///
#[derive(Block, Clone)]
pub struct ReadFile {
    /// The path to the file to read from.
    #[input]
    pub path: InputPort<String>,

    /// The output message stream.
    #[output]
    pub output: OutputPort<Bytes>,
}

impl ReadFile {
    pub fn new(path: InputPort<String>, output: OutputPort<Bytes>) -> Self {
        Self { path, output }
    }

    pub fn with_system(system: &System) -> Self {
        use crate::SystemBuilding;
        Self::new(system.input(), system.output())
    }
}

impl Block for ReadFile {
    fn execute(&mut self, _runtime: &dyn BlockRuntime) -> BlockResult {
        use std::io::prelude::Read;

        while let Some(path) = self.path.recv()? {
            let mut file = std::fs::OpenOptions::new().read(true).open(path)?;

            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            let bytes = Bytes::from(buffer);

            self.output.send(&bytes)?;
        }

        Ok(())
    }
}

#[cfg(feature = "std")]
impl StdioSystem for ReadFile {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        //use crate::{CoreBlocks, SysBlocks, SystemBuilding};

        config.allow_only(vec!["path"])?;

        Ok(System::build(|_s| todo!())) // TODO
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::ReadFile;
    use crate::{System, SystemBuilding, SystemExecution};

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.block(ReadFile::new(s.input(), s.output()));
        });
    }

    #[test]
    fn run_block() {
        use protoflow_core::{
            runtimes::StdRuntime as Runtime, transports::MpscTransport as Transport,
        };
        use std::io::Write;

        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        let test_content = "Hello, World!\n";
        temp_file.write_all(test_content.as_bytes()).unwrap();

        let mut system = System::new(&Runtime::new(Transport::new()).unwrap());
        let read_file = system.block(ReadFile::with_system(&system));

        let mut path = system.output();
        let output = system.input();

        system.connect(&path, &read_file.path);
        system.connect(&read_file.output, &output);

        let process = system.execute().unwrap();

        path.send(&temp_file.path().to_string_lossy().into())
            .unwrap();

        assert_eq!(
            output
                .recv()
                .expect("should receive output")
                .expect("output shouldn't be None"),
            test_content
        );

        path.close().unwrap();

        assert_eq!(
            output.recv(),
            Ok(None),
            "want EOS signal after path port is closed"
        );

        process.join().unwrap();
    }
}
