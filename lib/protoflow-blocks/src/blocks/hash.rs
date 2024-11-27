// This is free and unencumbered software released into the public domain.

mod hash;

#[cfg(not(any(
    feature = "hash-blake3",
    feature = "hash-md5",
    feature = "hash-sha1",
    feature = "hash-sha2"
)))]
mod inner {
    pub trait HashBlocks {}
    pub enum HashBlockConfig {}
}

#[cfg(any(
    feature = "hash-blake3",
    feature = "hash-md5",
    feature = "hash-sha1",
    feature = "hash-sha2"
))]
mod inner {
    use crate::{
        prelude::{vec, Box, Cow, Named, Vec},
        types::HashAlgorithm,
        BlockConnections, BlockInstantiation, InputPortName, OutputPortName, System,
    };
    use protoflow_core::Block;

    pub use super::hash::*;

    pub trait HashBlocks {
        fn hash(&mut self, algorithm: HashAlgorithm) -> Hash;

        #[cfg(feature = "hash-blake3")]
        fn hash_blake3(&mut self) -> Hash;

        #[cfg(feature = "hash-md5")]
        fn hash_md5(&mut self) -> Hash;

        #[cfg(feature = "hash-sha1")]
        fn hash_sha1(&mut self) -> Hash;

        #[cfg(feature = "hash-sha2")]
        fn hash_sha2(&mut self) -> Hash;
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub enum HashBlockTag {
        Hash,
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Debug)]
    pub enum HashBlockConfig {
        Hash {
            input: InputPortName,
            output: Option<OutputPortName>,
            hash: OutputPortName,
            algorithm: Option<HashAlgorithm>,
        },
    }

    impl Named for HashBlockConfig {
        fn name(&self) -> Cow<str> {
            use HashBlockConfig::*;
            Cow::Borrowed(match self {
                Hash { .. } => "Hash",
            })
        }
    }

    impl BlockConnections for HashBlockConfig {
        fn output_connections(&self) -> Vec<(&'static str, Option<OutputPortName>)> {
            use HashBlockConfig::*;
            match self {
                Hash { output, hash, .. } => {
                    vec![("output", output.clone()), ("hash", Some(hash.clone()))]
                }
            }
        }
    }

    impl BlockInstantiation for HashBlockConfig {
        fn instantiate(&self, system: &mut System) -> Box<dyn Block> {
            use HashBlockConfig::*;
            match self {
                Hash { algorithm, .. } => Box::new(super::Hash::with_system(system, *algorithm)),
            }
        }
    }
}

pub use inner::*;
