// This is free and unencumbered software released into the public domain.

use crate as protoflow;

use protoflow::derive::Block;
use protoflow::{Block, BlockError, BlockRuntime, Message, OutputPort};

//#[cfg(feature = "rand")]
//use rand::{distributions::Distribution, Rng};

/// A block for sending a random value.
#[derive(Block, Clone)]
pub struct Random<T: Message> {
    /// The port to send the value on.
    #[output]
    pub output: OutputPort<T>,

    /// A parameter for the random seed to use.
    #[parameter]
    #[allow(unused)]
    pub seed: Option<T>,
}

impl<T: Message> Block for Random<T> {
    fn execute(&mut self, runtime: &dyn BlockRuntime) -> Result<(), BlockError> {
        runtime.wait_for(&self.output)?;

        //self.output.send(todo!())?; // TODO

        Ok(())
    }
}
