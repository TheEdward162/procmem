//! Common definitions used across this library.

use std::num::NonZeroUsize;

/// Type to represent the offset of the address space.
///
/// This is basically the native pointer type, and we also assume it cannot be null.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[repr(transparent)]
pub struct OffsetType(NonZeroUsize);
impl OffsetType {
	pub fn new(offset: usize) -> Self {
		OffsetType(
			NonZeroUsize::new(offset)
				.expect("offset cannot be zero because it represents a valid pointer")
		)
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
		let value = unsafe {
			NonZeroUsize::new_unchecked(
				self.0.get().saturating_add(rhs)
			)
		};

		OffsetType(value)
	}
}
impl From<usize> for OffsetType {
	fn from(v: usize) -> Self {
		OffsetType::new(v)
	}
}
impl std::fmt::Display for OffsetType {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{:x}", self.get())
	}
}
