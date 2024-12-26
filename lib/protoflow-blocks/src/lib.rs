// This is free and unencumbered software released into the public domain.

#![no_std]
#![deny(unsafe_code)]

#[doc(hidden)]
pub use protoflow_core::prelude;

mod block_config;
pub use block_config::*;

mod block_connections;
pub use block_connections::*;

mod block_instantiation;
pub use block_instantiation::*;

mod block_tag;
pub use block_tag::*;

#[cfg(feature = "std")]
mod stdio;
#[cfg(feature = "std")]
pub use stdio::*;

mod system;
pub use system::*;

pub mod types;
pub use types::*;

pub use protoflow_core::{SystemBuilding, SystemExecution};

pub mod blocks {
    use super::*;

    include!("blocks/core.rs"); // CoreBlocks
    include!("blocks/flow.rs"); // FlowBlocks
    include!("blocks/hash.rs"); // HashBlocks
    include!("blocks/io.rs"); // IoBlocks
    include!("blocks/math.rs"); // MathBlocks
    include!("blocks/sys.rs"); // SysBlocks
    include!("blocks/text.rs"); // TextBlocks
}
pub use blocks::*;

pub trait AllBlocks:
    CoreBlocks + FlowBlocks + HashBlocks + IoBlocks + MathBlocks + SysBlocks + TextBlocks
{
}

#[cfg(feature = "std")]
#[doc(hidden)]
pub fn build_stdio_system(
    system_name: prelude::String,
    config: StdioConfig,
) -> Result<System, StdioError> {
    use prelude::String;
    Ok(match system_name.as_ref() {
        // CoreBlocks
        "Buffer" => Buffer::<String>::build_system(config)?,
        "Const" => Const::<String>::build_system(config)?,
        "Count" => Count::<String>::build_system(config)?,
        "Delay" => Delay::<String>::build_system(config)?,
        "Drop" => Drop::<String>::build_system(config)?,
        "Random" => Random::<u64>::build_system(config)?,
        // FlowBlocks
        // HashBlocks
        #[cfg(any(
            feature = "hash-blake3",
            feature = "hash-md5",
            feature = "hash-sha1",
            feature = "hash-sha2"
        ))]
        "Hash" => Hash::build_system(config)?,
        // IoBlocks
        "Decode" => Decode::build_system(config)?,
        "DecodeHex" => DecodeHex::build_system(config)?,
        "DecodeJSON" => DecodeJson::build_system(config)?,
        "Encode" => Encode::build_system(config)?,
        "EncodeHex" => EncodeHex::build_system(config)?,
        "EncodeJSON" => EncodeJson::build_system(config)?,
        // MathBlocks
        // SysBlocks
        "ReadDir" => ReadDir::build_system(config)?,
        "ReadEnv" => ReadEnv::<String>::build_system(config)?,
        "ReadFile" => ReadFile::build_system(config)?,
        #[cfg(feature = "serde")]
        "ReadSocket" => ReadSocket::build_system(config)?,
        "ReadStdin" => ReadStdin::build_system(config)?,
        "WriteFile" => WriteFile::build_system(config)?,
        #[cfg(feature = "serde")]
        "WriteSocket" => WriteSocket::build_system(config)?,
        "WriteStderr" => WriteStderr::build_system(config)?,
        "WriteStdout" => WriteStdout::build_system(config)?,
        // TextBlocks
        "ConcatStrings" => ConcatStrings::build_system(config)?,
        "DecodeCSV" => DecodeCsv::build_system(config)?,
        "EncodeCSV" => EncodeCsv::build_system(config)?,
        "SplitString" => SplitString::build_system(config)?,
        _ => return Err(StdioError::UnknownSystem(system_name))?,
    })
}
