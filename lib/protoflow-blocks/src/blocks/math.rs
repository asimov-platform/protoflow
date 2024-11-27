// This is free and unencumbered software released into the public domain.

use crate::{
    prelude::{Cow, Named},
    BlockConnections, BlockInstantiation,
};

pub trait MathBlocks {}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MathBlockTag {}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum MathBlockConfig {}

impl Named for MathBlockConfig {
    fn name(&self) -> Cow<str> {
        unreachable!()
    }
}

impl BlockConnections for MathBlockConfig {}

impl BlockInstantiation for MathBlockConfig {}
