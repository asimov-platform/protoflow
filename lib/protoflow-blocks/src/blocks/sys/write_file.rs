// This is free and unencumbered software released into the public domain.

extern crate std;

use crate::{
    prelude::{vec, Bytes, String},
    StdioConfig, StdioError, StdioSystem, System,
};
use protoflow_core::{Block, BlockResult, BlockRuntime, InputPort};
use protoflow_derive::Block;
use simple_mermaid::mermaid;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WriteFlags {
    pub create: bool,
    pub append: bool,
}

impl Default for WriteFlags {
    fn default() -> Self {
        Self {
            create: true,
            append: true,
        }
    }
}

/// A block that writes or appends bytes to the contents of a file.
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/sys/write_file.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/sys/write_file.seq.mmd" framed)]
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
/// $ protoflow execute WriteFile path=/tmp/file.txt
/// ```
///
#[derive(Block, Clone)]
pub struct WriteFile {
    /// The path to the file to write to.
    #[input]
    pub path: InputPort<String>,

    /// The input message stream.
    #[input]
    pub input: InputPort<Bytes>,

    #[parameter]
    pub flags: WriteFlags,
}

impl WriteFile {
    pub fn new(path: InputPort<String>, input: InputPort<Bytes>) -> Self {
        Self::with_params(path, input, None)
    }

    pub fn with_params(
        path: InputPort<String>,
        input: InputPort<Bytes>,
        flags: Option<WriteFlags>,
    ) -> Self {
        Self {
            path,
            input,
            flags: flags.unwrap_or_default(),
        }
    }

    pub fn with_system(system: &System, flags: Option<WriteFlags>) -> Self {
        use crate::SystemBuilding;
        Self::with_params(system.input(), system.input(), flags)
    }

    pub fn with_flags(self, flags: WriteFlags) -> Self {
        WriteFile { flags, ..self }
    }
}

impl Block for WriteFile {
    fn execute(&mut self, runtime: &dyn BlockRuntime) -> BlockResult {
        use std::io::prelude::Write;

        runtime.wait_for(&self.path)?;

        let Some(path) = self.path.recv()? else {
            return Ok(());
        };
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(self.flags.create)
            .append(self.flags.append)
            .truncate(!self.flags.append)
            .open(path)?;

        while let Some(message) = self.input.recv()? {
            file.write_all(&message)?;
        }

        drop(file);

        self.input.close()?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl StdioSystem for WriteFile {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        use crate::{CoreBlocks, SysBlocks, SystemBuilding};

        config.allow_only(vec!["path", "create", "append"])?;
        let path = config.get_string("path")?;

        let create = config.get_opt::<bool>("create")?;
        let append = config.get_opt::<bool>("append")?;

        let default_flags = WriteFlags::default();

        let flags = WriteFlags {
            create: create.unwrap_or(default_flags.create),
            append: append.unwrap_or(default_flags.append),
        };

        Ok(System::build(|s| {
            let stdin = config.read_stdin(s);
            let path_const = s.const_string(path);
            let write_file = s.write_file().with_flags(flags);

            s.connect(&path_const.output, &write_file.path);
            s.connect(&stdin.output, &write_file.input);
        }))
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use crate::{CoreBlocks, IoBlocks, SysBlocks, System, SystemBuilding, WriteFile};

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.block(WriteFile::with_system(s, None));
        });
    }

    #[test]
    #[ignore = "temporarily disabled"]
    fn run_block() {
        use std::{fs::File, io::Read, string::String};

        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("write-file-test.txt");

        // ok to fail:
        let _ = std::fs::remove_file(&output_path);

        System::run(|s| {
            let path = s.const_string(output_path.display());
            let content = s.const_string("Hello world!");
            let line_encoder = s.encode_lines();
            let write_file = s.write_file();
            s.connect(&content.output, &line_encoder.input);
            s.connect(&path.output, &write_file.path);
            s.connect(&line_encoder.output, &write_file.input);
        })
        .expect("system execution failed");

        let mut file = File::open(&output_path).expect("failed to open file for system output");

        let mut file_content = String::new();
        file.read_to_string(&mut file_content)
            .expect("failed to read system output");

        assert_eq!("Hello world!\n", file_content);

        drop(file);

        std::fs::remove_file(&output_path).expect("failed to remove temp file");
    }
}
