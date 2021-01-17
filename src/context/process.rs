use std::fs::{File, OpenOptions};
use std::io::{Seek, Write, SeekFrom};
use std::path::PathBuf;

use thiserror::Error;

use crate::map::MapEntry;

#[derive(Debug, Error)]
pub enum PtraceLockError {
	#[error("waitpid error {0}")]
	WaitpidError(std::io::Error),
	#[error("ptrace not attached")]
	NotAttached,
	#[error("cannot attach exclusively since the ptrace is already attached")]
	AlreadyAttached
}

#[derive(Debug, Error)]
pub enum WriteMemoryError {
	#[error(transparent)]
	PtraceLock(#[from] PtraceLockError),

	#[error("could not write")]
	Io(#[from] std::io::Error)

	// TODO
	// #[error("attempted to write to memory range that is not (wholly) mapped")]
	// RangeNotMapped
}

/// Context of one process.
///
/// Handles attaching and detaching ptraces, loading mappings and has readwrite access to the memory.
pub struct ProcessContext {
	pid: libc::pid_t,
	maps: Vec<MapEntry>,

	/// Number of times ptrace lock was requested.
	///
	/// Ptrace lock is not released until this number reaches 0 again.
	ptrace_lock: usize,

	/// The only readwrite copy of the file
	mem_rw: File,
}
impl ProcessContext {
	/// Attaches ptrace to the current process.
	///
	/// Returns false if ptrace is already attached (but still increases the internal counter).
	pub fn ptrace_lock(&mut self) -> Result<bool, PtraceLockError> {
		let mut result = false;
		
		if self.ptrace_lock == 1 {
			unsafe {
				self.ptrace_lock_raw()?;
			}
			result = true;
		}
		self.ptrace_lock += 1;

		Ok(result)
	}

	/// Detaches ptrace from the current process.
	///
	/// Returns false if ptrace is still attached (but still decreases the internal counter).
	pub fn ptrace_unlock(&mut self) -> Result<bool, PtraceLockError> {
		if self.ptrace_lock == 0 {
			return Err(PtraceLockError::NotAttached)
		}

		let mut result = false;

		self.ptrace_lock -= 1;
		if self.ptrace_lock == 0 {
			unsafe {
				self.ptrace_unlock_raw()?
			}
			result = true;
		}

		Ok(result)
	}

	pub fn ptrace_lock_exclusive(&mut self) -> Result<(), PtraceLockError> {
		if self.ptrace_lock > 0 {
			return Err(PtraceLockError::AlreadyAttached)
		}

		self.ptrace_lock()?;

		Ok(())
	}

	/// ## Safety
	/// * TODO: Manpages are evil
	unsafe fn ptrace_lock_raw(&mut self) -> Result<(), PtraceLockError> {
		libc::ptrace(libc::PTRACE_ATTACH, self.pid, 0, 0);
			
		if libc::waitpid(self.pid, std::ptr::null_mut(), 0) != 0 {
			return Err(PtraceLockError::WaitpidError(std::io::Error::last_os_error()))
		}

		Ok(())
	}

	/// ## Safety
	/// * TODO: Manpages are evil
	unsafe fn ptrace_unlock_raw(&mut self) -> Result<(), PtraceLockError> {
		libc::ptrace(libc::PTRACE_DETACH, self.pid, 0, 0);

		Ok(())
	}

	/// Safety
	/// * range must be mapped
	pub unsafe fn write_memory(&mut self, offset: usize, data: &[u8]) -> Result<(), WriteMemoryError> {
		self.ptrace_lock_exclusive()?;

		// TODO: Check memory range is mapped
		self.mem_rw.seek(SeekFrom::Start(offset as u64))?;
		self.mem_rw.write(data)?;

		self.ptrace_unlock()?;

		Ok(())
	}

	pub fn pid(&self) -> libc::pid_t {
		self.pid
	}

	pub fn maps(&self) -> &[MapEntry] {
		&self.maps
	}
}
impl Drop for ProcessContext {
	fn drop(&mut self) {
		if self.ptrace_lock > 0 {
			unsafe {
				self.ptrace_unlock_raw().expect("could not detach ptrace on drop")
			}
		}
	}
}