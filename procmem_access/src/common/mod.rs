//! Common definitions used across this library.

use std::num::NonZeroU64;
use std::convert::TryFrom;

/// Type to represent the offset of the address space.
///
/// This is basically the native pointer type, and we also assume it cannot be null.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[repr(transparent)]
pub struct OffsetType(NonZeroU64);
impl OffsetType {
	pub fn new(offset: u64) -> Option<Self> {
		Some(
			OffsetType(
				NonZeroU64::new(offset)?
			)
		)
	}

	pub fn new_unwrap(offset: u64) -> Self {
		Self::new(offset).expect("offset cannot be zero because it represents a valid pointer")
	}

	pub const fn get(&self) -> u64 {
		self.0.get()
	}

	pub const fn saturating_add(&self, rhs: u64) -> OffsetType {
		// Safe because we use saturating addition on one positive and non-negative number
		let value = unsafe { NonZeroU64::new_unchecked(self.0.get().saturating_add(rhs)) };

		OffsetType(value)
	}
}
impl TryFrom<u64> for OffsetType {
	type Error = std::num::TryFromIntError;

	fn try_from(value: u64) -> Result<Self, Self::Error> {
		Ok(
			OffsetType::from(NonZeroU64::try_from(value)?)
		)
	}
}
impl From<NonZeroU64> for OffsetType {
	fn from(offset: NonZeroU64) -> Self {
		OffsetType(offset)
	}
}
impl std::fmt::Display for OffsetType {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{:x}", self.get())
	}
}
