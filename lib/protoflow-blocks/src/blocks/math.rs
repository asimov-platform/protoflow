// This is free and unencumbered software released into the public domain.

pub mod math {
    use super::{
        prelude::{vec, Box, Cow, Named, Vec},
        BlockConnections, BlockInstantiation, InputPortName, OutputPortName, System,
    };
    use protoflow_core::Block;

    pub trait MathBlocks {
        fn add(&mut self) -> Add;
        fn div(&mut self) -> Div;
        fn mul(&mut self) -> Mul;
        fn sub(&mut self) -> Sub;
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub enum MathBlockTag {
        Add,
        Div,
        Mul,
        Sub,
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Debug)]
    pub enum MathBlockConfig {
        Add {
            input: InputPortName,
            output: OutputPortName,
        },
        Div {
            input: InputPortName,
            output: OutputPortName,
        },
        Mul {
            input: InputPortName,
            output: OutputPortName,
        },
        Sub {
            input: InputPortName,
            output: OutputPortName,
        },
    }

    impl Named for MathBlockConfig {
        fn name(&self) -> Cow<str> {
            use MathBlockConfig::*;
            Cow::Borrowed(match self {
                Add { .. } => "Add",
                Div { .. } => "Div",
                Mul { .. } => "Mul",
                Sub { .. } => "Sub",
            })
        }
    }

    impl BlockConnections for MathBlockConfig {
        fn output_connections(&self) -> Vec<(&'static str, Option<OutputPortName>)> {
            use MathBlockConfig::*;
            match self {
                Add { output, .. }
                | Div { output, .. }
                | Mul { output, .. }
                | Sub { output, .. } => {
                    vec![("output", Some(output.clone()))]
                }
            }
        }
    }

    impl BlockInstantiation for MathBlockConfig {
        fn instantiate(&self, system: &mut System) -> Box<dyn Block> {
            use MathBlockConfig::*;
            match self {
                Add { .. } => Box::new(super::Add::with_system(system)),
                Div { .. } => Box::new(super::Div::with_system(system)),
                Mul { .. } => Box::new(super::Mul::with_system(system)),
                Sub { .. } => Box::new(super::Sub::with_system(system)),
            }
        }
    }

    mod add;
    pub use add::*;

    mod div;
    pub use div::*;

    mod mul;
    pub use mul::*;

    mod sub;
    pub use sub::*;
}

pub use math::*;
