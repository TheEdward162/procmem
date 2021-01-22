use thiserror::Error;

use crate::common::OffsetType;

#[derive(Debug, Error)]
pub enum LockError {
	#[error("ptrace(PTRACE_ATTACH) failed")]
	PtraceError(std::io::Error),
	#[error("waitpid failed")]
	WaitpidError(std::io::Error)
}

#[derive(Debug, Error)]
pub enum ExclusiveLockError {
	#[error("process is already locked")]
	AlreadyLocked,
	#[error(transparent)]
	LockError(#[from] LockError)
}

#[derive(Debug, Error)]
pub enum UnlockError {
	#[error("process is not locked")]
	NotLocked,
	#[error("ptrace(PTRACE_DETACH) failed")]
	PtraceError(std::io::Error),
}

#[derive(Debug, Error)]
pub enum ReadError {
	#[error("not permitted to read from this range")]
	NotPermitted,
	#[error("could not perform memory read")]
	Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum WriteError {
	#[error("not permitted to write to this range")]
	NotPermitted,
	#[error("could not perform memory write")]
	Io(#[from] std::io::Error),
}

/// Trait implemented on abstractions over accessing process memory.
pub trait MemoryAccess {
	/// Recursively lock the process.
	///
	/// Reading from the process memory without locking it may cause data races.
	///
	/// Return `true` if the lock was acquired in this call (as opposed to just increasing the counter).
	fn lock(&mut self) -> Result<bool, LockError>;

	/// Exclusively locks the process.
	///
	/// Writing to the process memory without exclusively locking it may cause data races.
	fn lock_exlusive(&mut self) -> Result<(), ExclusiveLockError>;

	/// Recursively unlock the process.
	///
	/// Should be called once for each [`lock`](MemoryAccess::lock) to unlock.
	///
	/// Returns `true< if the lock was released in this call (as opposed to just decreasing the counter).
	fn unlock(&mut self) -> Result<bool, UnlockError>;

	/// Read exact amount of bytes to fill the `buffer` from `offset`.
	///
	/// ## Safety
	/// * The process must be locked and or otherwise protected against data races.
	/// * Offset must be mapped in the process memory mappings.
	unsafe fn read(&mut self, offset: OffsetType, buffer: &mut [u8]) -> Result<(), ReadError>;

	/// Write exact amount of bytes from `data` into the process memory starting at `offset`.
	///
	/// ## Safety
	/// * The process must be exclusively locked or otherwise protected against data races.
	/// * Offset must be mapped in the process memory mappings.
	unsafe fn write(&mut self, offset: OffsetType, data: &[u8]) -> Result<(), WriteError>;
}