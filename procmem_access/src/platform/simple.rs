//! This module contains best-effort abstraction over platform specific implementations
//! in the parent module.
//!
//! For each supported platform, this module exports uniformly named types and functions
//! for simple common functionality.

#[cfg(target_os = "linux")]
mod inner {
	use super::super::{ptrace, procfs};

	pub type SimpleMemoryLock = ptrace::PtraceLock;
	pub type SimpleMemoryAccess = procfs::ProcfsAccess;
	pub type SimpleMemoryMap = procfs::ProcfsMemoryMap;

	pub use procfs::ProcessInfo;
}

#[cfg(target_os = "macos")]
mod inner {
	use super::super::{ptrace, mach as mch};

	pub type SimpleMemoryLock = ptrace::PtraceLock;
	pub type SimpleMemoryAccess = mch::MachAccess;
	pub type SimpleMemoryMap = mch::MachMemoryMap;

	pub use mch::ProcessInfo;
}

#[cfg(target_os = "windows")]
mod inner {
	// TODO
}

pub use inner::{SimpleMemoryLock, SimpleMemoryAccess, SimpleMemoryMap, ProcessInfo};
