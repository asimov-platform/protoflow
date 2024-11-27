// This is free and unencumbered software released into the public domain.

use crate::{
    prelude::{Cow, Named},
    BlockConnections, BlockInstantiation,
};

pub trait FlowBlocks {}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FlowBlockTag {}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum FlowBlockConfig {}

impl Named for FlowBlockConfig {
    fn name(&self) -> Cow<str> {
        unreachable!()
    }
}

impl BlockConnections for FlowBlockConfig {}

impl BlockInstantiation for FlowBlockConfig {}
