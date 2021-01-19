pub mod merge;

/// Type to represent the offset of the address space.
///
/// This is basically the pointer type, and we also assume it cannot be null.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[repr(transparent)]
pub struct OffsetType(std::num::NonZeroUsize);
impl OffsetType {
	pub fn new(offset: usize) -> Self {
		OffsetType(
			std::num::NonZeroUsize::new(offset)
				.expect("offset cannot be zero because it represents a valid pointer")
		)
	}

	pub fn get(&self) -> usize {
		self.0.get()
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
