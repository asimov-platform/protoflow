// This is free and unencumbered software released into the public domain.

use super::{prelude::Vec, InputPortName, OutputPortName};

/// A trait for defining the connections of a block instance.
pub trait BlockConnections {
    fn input_connections(&self) -> Vec<(&'static str, Option<InputPortName>)> {
        unimplemented!()
    }

    fn output_connections(&self) -> Vec<(&'static str, Option<OutputPortName>)> {
        unimplemented!()
    }
}