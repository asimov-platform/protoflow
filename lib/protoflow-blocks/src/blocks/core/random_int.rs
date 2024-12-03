// This is free and unencumbered software released into the public domain.

use crate::{
    prelude::{format, vec},
    StdioConfig, StdioError, StdioSystem, System,
};
use protoflow_core::{Block, BlockError, BlockResult, BlockRuntime, OutputPort};
use protoflow_derive::Block;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use simple_mermaid::mermaid;
/// A block for generating and sending a random number.
///
/// # Block Diagram
#[doc = mermaid!("../../../doc/core/random_int.mmd")]
///
/// # Sequence Diagram
#[doc = mermaid!("../../../doc/core/random_int.seq.mmd" framed)]
///
/// # Examples
///
/// ## Using the block in a system
///
/// ```rust
/// # use protoflow_blocks::*;
/// # fn main() {
/// System::build(|s| {
///     let config = StdioConfig {
///         encoding: Default::default(),
///         params: Default::default(),
///     };
///     let random_int = s.random_int();
///     let number_encoder = s.encode_with::<i64>(config.encoding);
///     let stdout = s.write_stdout();
///     s.connect(&random_int.output, &number_encoder.input);
///     s.connect(&number_encoder.output, &stdout.input);
/// });
/// # }
/// ```
///
/// ## Running the block via the CLI
///
/// ```console
/// $ protoflow execute RandomInt
/// ```
///
/// ```console
/// $ protoflow execute RandomInt seed=42 min=0 max=100
/// ```
///
#[derive(Block, Clone)]
pub struct RandomInt {
    /// The port to send the value on.
    #[output]
    pub output: OutputPort<i64>,
    /// A parameter for the random seed to use.
    #[parameter]
    pub seed: Option<u64>,
    /// A parameter for the random min to use.
    #[parameter]
    pub min: Option<i64>,
    /// A parameter for the random max to use.
    #[parameter]
    pub max: Option<i64>,
}

impl RandomInt {
    pub fn new(output: OutputPort<i64>) -> Self {
        Self::with_params(output, None, None, None)
    }

    pub fn with_params(
        output: OutputPort<i64>,
        seed: Option<u64>,
        min: Option<i64>,
        max: Option<i64>,
    ) -> Self {
        Self {
            output,
            seed,
            min,
            max,
        }
    }

    pub fn with_system(
        system: &System,
        seed: Option<u64>,
        min: Option<i64>,
        max: Option<i64>,
    ) -> Self {
        use crate::SystemBuilding;
        Self::with_params(system.output(), seed, min, max)
    }
}

impl Block for RandomInt {
    fn execute(&mut self, runtime: &dyn BlockRuntime) -> BlockResult {
        runtime.wait_for(&self.output)?;

        let mut rng = if let Some(seed) = self.seed {
            StdRng::seed_from_u64(seed)
        } else {
            StdRng::from_rng(rand::thread_rng())
        };

        let min = self.min.unwrap_or(i64::MIN);
        let max = self.max.unwrap_or(i64::MAX);

        if min >= max {
            return Err(BlockError::Other(format!(
                "Invalid range: min ({}) must be less than max ({})",
                min, max
            )));
        }

        let random_value = rng.gen_range(min..max);

        self.output.send(&random_value)?;

        Ok(())
    }
}

#[cfg(feature = "std")]
impl StdioSystem for RandomInt {
    fn build_system(config: StdioConfig) -> Result<System, StdioError> {
        use crate::{CoreBlocks, IoBlocks, SystemBuilding};

        config.allow_only(vec!["seed", "min", "max"])?;
        let seed = config.get_opt::<u64>("seed")?;
        let min = config.get_opt::<i64>("min")?;
        let max = config.get_opt::<i64>("max")?;

        Ok(System::build(|s| {
            let random_int = s.random_int_with_params(seed, min, max);
            let number_encoder = s.encode_with::<i64>(config.encoding);
            let stdout = config.write_stdout(s);
            s.connect(&random_int.output, &number_encoder.input);
            s.connect(&number_encoder.output, &stdout.input);
        }))
    }
}

#[cfg(test)]
mod tests {

    use super::RandomInt;
    use crate::{System, SystemBuilding};

    #[test]
    fn instantiate_block() {
        // Check that the block is constructible:
        let _ = System::build(|s| {
            let _ = s.block(RandomInt::new(s.output()));
        });
    }
}
