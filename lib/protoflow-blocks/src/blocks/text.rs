// This is free and unencumbered software released into the public domain.

use crate::{
    prelude::{vec, Box, Cow, Named, String, Vec},
    BlockConnections, BlockInstantiation, InputPortName, OutputPortName, System,
};
use protoflow_core::Block;

pub trait TextBlocks {
    fn concat_strings(&mut self) -> ConcatStrings;
    fn concat_strings_by(&mut self, delimiter: &str) -> ConcatStrings;
    fn decode_csv(&mut self) -> DecodeCsv;
    fn encode_csv(&mut self) -> EncodeCsv;
    fn split_string(&mut self, delimiter: &str) -> SplitString;
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TextBlockTag {
    ConcatStrings,
    DecodeCsv,
    EncodeCsv,
    SplitString,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum TextBlockConfig {
    ConcatStrings {
        input: InputPortName,
        output: OutputPortName,
        delimiter: Option<String>,
    },
    #[cfg_attr(feature = "serde", serde(rename = "DecodeCSV"))]
    DecodeCsv {
        input: InputPortName,
        header: OutputPortName,
        rows: OutputPortName,
    },
    #[cfg_attr(feature = "serde", serde(rename = "EncodeCSV"))]
    EncodeCsv {
        header: InputPortName,
        rows: InputPortName,
        output: OutputPortName,
    },
    SplitString {
        input: InputPortName,
        output: OutputPortName,
        delimiter: Option<String>,
    },
}

impl Named for TextBlockConfig {
    fn name(&self) -> Cow<str> {
        use TextBlockConfig::*;
        Cow::Borrowed(match self {
            ConcatStrings { .. } => "ConcatStrings",
            DecodeCsv { .. } => "DecodeCSV",
            EncodeCsv { .. } => "EncodeCSV",
            SplitString { .. } => "SplitString",
        })
    }
}

impl BlockConnections for TextBlockConfig {
    fn output_connections(&self) -> Vec<(&'static str, Option<OutputPortName>)> {
        use TextBlockConfig::*;
        match self {
            ConcatStrings { output, .. }
            | EncodeCsv { output, .. }
            | SplitString { output, .. } => {
                vec![("output", Some(output.clone()))]
            }
            DecodeCsv { header, rows, .. } => {
                vec![
                    ("header", Some(header.clone())),
                    ("rows", Some(rows.clone())),
                ]
            }
        }
    }
}

impl BlockInstantiation for TextBlockConfig {
    fn instantiate(&self, system: &mut System) -> Box<dyn Block> {
        use TextBlockConfig::*;
        match self {
            ConcatStrings { delimiter, .. } => {
                Box::new(super::ConcatStrings::with_system(system, delimiter.clone()))
            }
            DecodeCsv { .. } => Box::new(super::DecodeCsv::with_system(system)),
            EncodeCsv { .. } => Box::new(super::EncodeCsv::with_system(system)),
            SplitString { delimiter, .. } => {
                Box::new(super::SplitString::with_system(system, delimiter.clone()))
            }
        }
    }
}

mod concat_strings;
pub use concat_strings::*;

mod decode_csv;
pub use decode_csv::*;

mod encode_csv;
pub use encode_csv::*;

mod split_string;
pub use split_string::*;
