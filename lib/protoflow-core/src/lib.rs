// This is free and unencumbered software released into the public domain.

#![no_std]
#![allow(unsafe_code)] // for `impl Sync`

extern crate self as protoflow_core;

#[doc(hidden)]
pub mod prelude;

mod block;
pub use block::*;

#[cfg(feature = "tokio")]
mod async_block;
#[cfg(feature = "tokio")]
pub use async_block::*;

mod block_descriptor;
pub use block_descriptor::*;

mod block_error;
pub use block_error::*;

mod block_runtime;
pub use block_runtime::*;

mod function_block;
pub use function_block::*;

mod input_port;
pub use input_port::*;

mod input_ports;
pub use input_ports::*;

mod message;
pub use message::*;

mod message_buffer;
pub use message_buffer::*;

mod message_receiver;
pub use message_receiver::*;

mod message_sender;
pub use message_sender::*;

mod output_port;
pub use output_port::*;

mod output_ports;
pub use output_ports::*;

mod parameter_descriptor;
pub use parameter_descriptor::*;

mod port;
pub use port::*;

mod port_descriptor;
pub use port_descriptor::*;

mod port_error;
pub use port_error::*;

mod port_id;
pub use port_id::*;

mod port_state;
pub use port_state::*;

mod process;
pub use process::*;

mod runtime;
pub use runtime::*;

pub mod runtimes;

mod system;
pub use system::*;

mod transport;
pub use transport::*;

pub mod transports;

#[allow(unused_imports)]
pub(crate) mod utils {
    mod rw_condvar;
    pub use rw_condvar::*;
}

pub use prost_types as types;

pub use prost::DecodeError;

#[cfg(feature = "tracing")]
#[doc(hidden)]
mod tracing {
    pub use tracing::{debug, error, info, trace, warn};
}

#[cfg(not(feature = "tracing"))]
#[doc(hidden)]
#[rustfmt::skip]
mod tracing {
    #[macro_export] macro_rules! debug { ($($arg:tt)+) => (); }
    #[macro_export] macro_rules! error { ($($arg:tt)+) => (); }
    #[macro_export] macro_rules! info { ($($arg:tt)+) => (); }
    #[macro_export] macro_rules! trace { ($($arg:tt)+) => (); }
    #[macro_export] macro_rules! warn { ($($arg:tt)+) => (); }
}

#[allow(unused)]
pub use tracing::*;
