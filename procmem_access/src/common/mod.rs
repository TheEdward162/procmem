//! Common definitions used across this library.

use std::num::NonZeroUsize;
use std::convert::TryFrom;

/// Type to represent the offset of the address space.
///
/// This is basically the native pointer type, and we also assume it cannot be null.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[repr(transparent)]
pub struct OffsetType(NonZeroUsize);
impl OffsetType {
	pub fn new(offset: usize) -> Option<Self> {
		Some(
			OffsetType(
				NonZeroUsize::new(offset)?
			)
		)
	}

	pub fn new_unwrap(offset: usize) -> Self {
		Self::new(offset).expect("offset cannot be zero because it represents a valid pointer")
	}

	pub fn try_new(offset: usize) -> Option<Self> {
		match NonZeroUsize::new(offset) {
			None => None,
			Some(n) => Some(Self(n))
		}
	}

	pub const fn get(&self) -> usize {
		self.0.get()
	}

	pub const fn saturating_add(&self, rhs: usize) -> OffsetType {
		// Safe because we use saturating addition on one positive and non-negative number
		let value = unsafe { NonZeroUsize::new_unchecked(self.0.get().saturating_add(rhs)) };

		OffsetType(value)
	}
}
impl TryFrom<usize> for OffsetType {
	type Error = std::num::TryFromIntError;

	fn try_from(value: usize) -> Result<Self, Self::Error> {
		Ok(
			OffsetType::from(NonZeroUsize::try_from(value)?)
		)
	}
}
impl From<NonZeroUsize> for OffsetType {
	fn from(offset: NonZeroUsize) -> Self {
		OffsetType(offset)
	}
}
impl std::fmt::Display for OffsetType {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{:x}", self.get())
	}
}
