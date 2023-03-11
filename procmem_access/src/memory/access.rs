use thiserror::Error;

use crate::common::OffsetType;

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

/// Trait implemented on abstractions over reading and writing from memory.
pub trait MemoryAccess {
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
