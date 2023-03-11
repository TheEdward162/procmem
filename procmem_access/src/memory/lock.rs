use thiserror::Error;

#[derive(Debug, Error)]
pub enum LockError {
	#[error("process is already locked exclusively")]
	AlreadyLocked,
	#[error("platform specific error: {0}")]
	PlatformError(Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, Error)]
pub enum UnlockError {
	#[error("process is not locked")]
	NotLocked,
	#[error("platform specific error: {0}")]
	PlatformError(Box<dyn std::error::Error + Send + Sync>),
}

/// Trait implemented on abstractions over locking and unlocking process memory.
pub trait MemoryLock {
	/// Recursively lock the process.
	///
	/// Reading from the process memory without locking it may cause data races.
	///
	/// Return `true` if the lock was acquired in this call (as opposed to just increasing the counter).
	fn lock(&mut self) -> Result<bool, LockError>;

	/// Exclusively locks the process.
	///
	/// Writing to the process memory without exclusively locking it may cause data races.
	fn lock_exlusive(&mut self) -> Result<(), LockError>;

	/// Recursively unlock the process.
	///
	/// Should be called once for each [`lock`](MemoryAccess::lock) to unlock.
	///
	/// Returns `true` if the lock was released in this call (as opposed to just decreasing the counter).
	fn unlock(&mut self) -> Result<bool, UnlockError>;
}
