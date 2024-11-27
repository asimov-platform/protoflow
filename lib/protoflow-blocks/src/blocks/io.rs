// This is free and unencumbered software released into the public domain.

use crate::{
    prelude::{vec, Box, Cow, FromStr, Named, String, ToString, Vec},
    types::Encoding,
    BlockConnections, BlockInstantiation, InputPortName, OutputPortName, System,
};
use protoflow_core::{Block, Message};

pub trait IoBlocks {
    fn decode<T: Message + FromStr + 'static>(&mut self) -> Decode<T>;
    fn decode_hex(&mut self) -> DecodeHex;
    fn decode_json(&mut self) -> DecodeJson;
    fn decode_with<T: Message + FromStr + 'static>(&mut self, encoding: Encoding) -> Decode<T>;

    fn decode_lines<T: Message + FromStr + 'static>(&mut self) -> Decode<T> {
        self.decode_with::<T>(Encoding::TextWithNewlineSuffix)
    }

    fn encode<T: Message + ToString + 'static>(&mut self) -> Encode<T>;
    fn encode_with<T: Message + ToString + 'static>(&mut self, encoding: Encoding) -> Encode<T>;

    fn encode_lines<T: Message + ToString + 'static>(&mut self) -> Encode<T> {
        self.encode_with::<T>(Encoding::TextWithNewlineSuffix)
    }

    fn encode_hex(&mut self) -> EncodeHex;

    fn encode_json(&mut self) -> EncodeJson;
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IoBlockTag {
    Decode,
    DecodeHex,
    DecodeJson,
    Encode,
    EncodeHex,
    EncodeJson,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum IoBlockConfig {
    Decode {
        input: InputPortName,
        output: OutputPortName,
        encoding: Option<Encoding>,
    },
    DecodeHex {
        input: InputPortName,
        output: OutputPortName,
    },
    #[cfg_attr(feature = "serde", serde(rename = "DecodeJSON"))]
    DecodeJson {
        input: InputPortName,
        output: OutputPortName,
    },

    Encode {
        input: InputPortName,
        output: OutputPortName,
        encoding: Option<Encoding>,
    },

    EncodeHex {
        input: InputPortName,
        output: OutputPortName,
    },

    #[cfg_attr(feature = "serde", serde(rename = "EncodeJSON"))]
    EncodeJson {
        input: InputPortName,
        output: OutputPortName,
    },
}

impl Named for IoBlockConfig {
    fn name(&self) -> Cow<str> {
        use IoBlockConfig::*;
        Cow::Borrowed(match self {
            Decode { .. } => "Decode",
            DecodeHex { .. } => "DecodeHex",
            DecodeJson { .. } => "DecodeJSON",
            Encode { .. } => "Encode",
            EncodeHex { .. } => "EncodeHex",
            EncodeJson { .. } => "EncodeJSON",
        })
    }
}

impl BlockConnections for IoBlockConfig {
    fn output_connections(&self) -> Vec<(&'static str, Option<OutputPortName>)> {
        use IoBlockConfig::*;
        match self {
            Decode { output, .. }
            | DecodeHex { output, .. }
            | DecodeJson { output, .. }
            | Encode { output, .. }
            | EncodeHex { output, .. }
            | EncodeJson { output, .. } => {
                vec![("output", Some(output.clone()))]
            }
        }
    }
}

impl BlockInstantiation for IoBlockConfig {
    fn instantiate(&self, system: &mut System) -> Box<dyn Block> {
        use IoBlockConfig::*;
        match self {
            Decode { encoding, .. } => {
                Box::new(super::Decode::<String>::with_system(system, *encoding))
            }
            DecodeHex { .. } => Box::new(super::DecodeHex::with_system(system)),
            DecodeJson { .. } => Box::new(super::DecodeJson::with_system(system)),
            Encode { encoding, .. } => {
                Box::new(super::Encode::<String>::with_system(system, *encoding))
            }
            EncodeHex { .. } => Box::new(super::EncodeHex::with_system(system)),
            EncodeJson { .. } => Box::new(super::EncodeJson::with_system(system)),
        }
    }
}

mod decode;
pub use decode::*;

mod decode_hex;
pub use decode_hex::*;

mod decode_json;
pub use decode_json::*;

mod encode;
pub use encode::*;

mod encode_hex;
pub use encode_hex::*;

mod encode_json;
pub use encode_json::*;
