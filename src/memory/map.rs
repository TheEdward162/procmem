use std::path::PathBuf;

use crate::common::OffsetType;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MemoryPagePermissions {
	bits: u8
}
impl MemoryPagePermissions {
	pub const MASK_EXEC: u8 = 1 << 2;
	pub const MASK_READ: u8 = 1 << 0;
	pub const MASK_SHARE: u8 = 1 << 3;
	pub const MASK_WRITE: u8 = 1 << 1;

	pub fn new(
		read: bool,
		write: bool,
		exec: bool,
		share: bool
	) -> Self {
		MemoryPagePermissions {
			bits: (
				read as u8 * Self::MASK_READ
			) | (
				write as u8 * Self::MASK_WRITE
			) | (
				exec as u8 * Self::MASK_WRITE
			) | (
				share as u8 * Self::MASK_WRITE
			)
		}
	}

	pub fn read(&self) -> bool {
		self.bits & Self::MASK_READ != 0
	}

	pub fn write(&self) -> bool {
		self.bits & Self::MASK_WRITE != 0
	}

	pub fn exec(&self) -> bool {
		self.bits & Self::MASK_EXEC != 0
	}

	pub fn shared(&self) -> bool {
		self.bits & Self::MASK_SHARE != 0
	}
}
impl std::fmt::Display for MemoryPagePermissions {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"{}{}{}{}",
			if self.read() { 'r' } else { '-' },
			if self.write() { 'w' } else { '-' },
			if self.exec() { 'x' } else { '-' },
			if self.shared() { 's' } else { 'p' },
		)
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum MemoryPageType {
	/// The API does not provide additional information.
	Unknown,

	/// Main thread stack.
	Stack,
	/// Process heap.
	Heap,
	/// Anonymous mapping.
	Anon,
	/// File-backed mapping
	File(PathBuf),

	// TODO: SelfFile or something similar

	// TODO: Research platforms more
	// Vvar,
	// Vdso,
}
impl std::fmt::Display for MemoryPageType {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			MemoryPageType::Unknown => write!(f, "[unknown]"),
			MemoryPageType::Stack => write!(f, "[stack]"),
			MemoryPageType::Heap => write!(f, "[heap]"),
			MemoryPageType::Anon => write!(f, ""),
			MemoryPageType::File(path) => write!(f, "{}", path.display()),
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryPage {
	pub address_range: [OffsetType; 2],
	pub permissions: MemoryPagePermissions,
	pub page_type: MemoryPageType
}
impl std::fmt::Display for MemoryPage {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"{}-{} {} {}",
			self.address_range[0], self.address_range[1], self.permissions, self.page_type
		)
	}
}

pub trait MemoryMap {
	/// Returns an ordered slice of memory pages.
	fn pages(&self) -> &[MemoryPage];

	/// Returns the mapped memory page within which the given offset falls.
	fn page(&self, offset: OffsetType) -> Option<&MemoryPage>;
}