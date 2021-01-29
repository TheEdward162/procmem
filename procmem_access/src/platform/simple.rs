//! This module contains best-effort abstraction over platform specific implementations
//! in the parent module.
//!
//! For each supported platform, this module exports uniformly named types and functions
//! for simple common functionality.

#[cfg(target_os = "linux")]
mod inner {
	pub type SimpleMemoryLock = super::super::ptrace::PtraceLock;
	pub type SimpleMemoryAccess = super::super::procfs::ProcfsMemoryAccess;
	pub type SimpleMemoryMap = super::super::procfs::ProcfsMemoryMap;
}

#[cfg(target_os = "macos")]
mod inner {
	pub type SimpleMemoryLock = super::super::ptrace::PtraceLock;
	pub type SimpleMemoryAccess = ();
	pub type SimpleMemoryMap = ();
}

#[cfg(target_os = "windows")]
mod inner {
	// TODO
}

pub use inner::{SimpleMemoryLock, SimpleMemoryAccess, SimpleMemoryMap};