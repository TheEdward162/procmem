//! Provides JSON RPC interface and implementation for procmem access and scan libraries.
//!
//! This library can also be used for interface definitions only by disabling
//! the `implementation` feature (disabling the defaults features). It does not provide
//! implementation of communiation channels.

pub mod rpc;
pub mod procedures;

