use std::{
	fs::{File, OpenOptions},
	io::{Read, Seek, SeekFrom, Write},
};

use thiserror::Error;

use crate::{
	common::OffsetType,
	memory::access::{MemoryAccess, ReadError, WriteError},
};

#[derive(Debug, Error)]
pub enum ProcfsAccessError {
	#[error("could not open memory file")]
	MemoryIo(std::io::Error),
}

/// Procfs implementation of memory access.
///
/// Uses `ptrace` to lock (stop) the process. Ptrace is attached only the first time a lock is acquired, not when the process is opened.
///
/// Ptrace is detached on drop.
pub struct ProcfsAccess {
	#[allow(dead_code)]
	pid: libc::pid_t,
	mem: File,
}
impl ProcfsAccess {
	pub fn mem_path(pid: libc::pid_t) -> std::path::PathBuf {
		format!("/proc/{}/mem", pid).into()
	}

	/// Opens a process with given `pid`.
	///
	/// The process memory access file is located in `/proc/[pid]/mem`.
	pub fn new(pid: libc::pid_t) -> Result<Self, ProcfsAccessError> {
		let path = Self::mem_path(pid);

		let mem = OpenOptions::new()
			.read(true)
			.write(true)
			.open(path)
			.map_err(|err| ProcfsAccessError::MemoryIo(err))?;

		Ok(ProcfsAccess { pid, mem })
	}
}
impl MemoryAccess for ProcfsAccess {
	unsafe fn read(&mut self, offset: OffsetType, buffer: &mut [u8]) -> Result<(), ReadError> {
		self.mem.seek(SeekFrom::Start(offset.get() as u64))?;

		self.mem.read_exact(buffer)?;

		Ok(())
	}

	unsafe fn write(&mut self, offset: OffsetType, data: &[u8]) -> Result<(), WriteError> {
		self.mem.seek(SeekFrom::Start(offset.get() as u64))?;

		self.mem.write_all(data)?;

		Ok(())
	}
}
