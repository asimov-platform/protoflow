// This is free and unencumbered software released into the public domain.

pub mod core {
    use super::{
        prelude::{vec, Box, Bytes, Cow, Named, Vec},
        BlockConnections, BlockInstantiation, InputPortName, OutputPortName, System,
    };
    use crate::{
        prelude::{Duration, Range, String, ToString},
        types::DelayType,
    };
    use protoflow_core::{Block, Message};

    pub trait CoreBlocks {
        fn buffer<T: Message + Into<T> + 'static>(&mut self) -> Buffer<T>;

        fn const_bytes<T: Into<Bytes>>(&mut self, value: T) -> Const<Bytes>;

        fn const_string(&mut self, value: impl ToString) -> Const<String>;

        fn count<T: Message + 'static>(&mut self) -> Count<T>;

        fn delay<T: Message + 'static>(&mut self) -> Delay<T>;

        fn delay_by<T: Message + 'static>(&mut self, delay: DelayType) -> Delay<T>;

        fn delay_by_fixed<T: Message + 'static>(&mut self, delay: Duration) -> Delay<T> {
            self.delay_by(DelayType::Fixed(delay))
        }

        fn delay_by_random<T: Message + 'static>(&mut self, delay: Range<Duration>) -> Delay<T> {
            self.delay_by(DelayType::Random(delay))
        }

        fn drop<T: Message + 'static>(&mut self) -> Drop<T>;

        fn map_from<Input: Message + 'static, Output: Message + From<Input> + 'static>(
            &mut self,
        ) -> MapFrom<Input, Output>;

        fn random<T: Message + 'static>(&mut self) -> Random<T>;

        fn random_seeded<T: Message + 'static>(&mut self, seed: Option<u64>) -> Random<T>;
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub enum CoreBlockTag {
        Buffer,
        Const,
        Count,
        Delay,
        Drop,
        MapFrom,
        Random,
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Debug)]
    pub enum CoreBlockConfig {
        Buffer {
            input: InputPortName,
        },

        Const {
            output: OutputPortName,
            value: String,
        },

        Count {
            input: InputPortName,
            output: Option<OutputPortName>,
            count: OutputPortName,
        },

        Delay {
            input: InputPortName,
            output: OutputPortName,
            delay: Option<DelayType>,
        },

        Drop {
            input: InputPortName,
        },

        MapFrom {
            input: InputPortName,
            output: OutputPortName,
        },

        Random {
            output: OutputPortName,
            seed: Option<u64>,
        },
    }

    impl Named for CoreBlockConfig {
        fn name(&self) -> Cow<str> {
            use CoreBlockConfig::*;
            Cow::Borrowed(match self {
                Buffer { .. } => "Buffer",
                Const { .. } => "Const",
                Count { .. } => "Count",
                Delay { .. } => "Delay",
                Drop { .. } => "Drop",
                MapFrom { .. } => "MapFrom",
                Random { .. } => "Random",
            })
        }
    }

    impl BlockConnections for CoreBlockConfig {
        fn output_connections(&self) -> Vec<(&'static str, Option<OutputPortName>)> {
            use CoreBlockConfig::*;
            match self {
                Buffer { .. } => vec![],
                Const { output, .. } => vec![("output", Some(output.clone()))],
                Count { output, count, .. } => {
                    vec![("output", output.clone()), ("count", Some(count.clone()))]
                }
                Delay { output, .. } => vec![("output", Some(output.clone()))],
                Drop { .. } => vec![],
                MapFrom { output, .. } => vec![("output", Some(output.clone()))],
                Random { output, .. } => vec![("output", Some(output.clone()))],
            }
        }
    }

    impl BlockInstantiation for CoreBlockConfig {
        fn instantiate(&self, system: &mut System) -> Box<dyn Block> {
            use super::SystemBuilding;
            use CoreBlockConfig::*;
            match self {
                Buffer { .. } => Box::new(super::Buffer::new(system.input_any())), // TODO: Buffer::with_system(system)
                Const { value, .. } => Box::new(super::Const::with_system(system, value.clone())),
                Count { .. } => Box::new(super::Count::new(
                    system.input_any(),
                    system.output(),
                    system.output(),
                )), // TODO: Count::with_system(system)
                Delay { delay, .. } => {
                    Box::new(super::Delay::with_params(
                        system.input_any(),
                        system.output(),
                        delay.clone(),
                    ))
                    // TODO: Delay::with_system(system, Some(delay.clone())))
                }
                Drop { .. } => Box::new(super::Drop::new(system.input_any())), // TODO: Drop::with_system(system)
                MapFrom { .. } => Box::new(super::MapFrom::with_params(
                    system.input_any(),
                    system.output_any(),
                )),
                Random { seed, .. } => {
                    Box::new(super::Random::with_params(system.output::<u64>(), *seed))
                    // TODO: Random::with_system(system, *seed))
                }
            }
        }
    }

    mod buffer;
    pub use buffer::*;

    mod r#const;
    pub use r#const::*;

    mod count;
    pub use count::*;

    mod delay;
    pub use delay::*;

    mod drop;
    pub use drop::*;

    mod map_from;
    pub use map_from::*;

    mod random;
    pub use random::*;
}

pub use core::*;
