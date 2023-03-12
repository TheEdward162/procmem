use std::path::PathBuf;

use crate::{common::OffsetType, util::AccFilter};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MemoryPagePermissions {
	bits: u8,
}
impl MemoryPagePermissions {
	pub const MASK_EXEC: u8 = 1 << 2;
	pub const MASK_READ: u8 = 1 << 0;
	pub const MASK_SHARE: u8 = 1 << 3;
	pub const MASK_WRITE: u8 = 1 << 1;

	pub const fn new(read: bool, write: bool, exec: bool, share: bool) -> Self {
		MemoryPagePermissions {
			bits: (read as u8 * Self::MASK_READ)
				| (write as u8 * Self::MASK_WRITE)
				| (exec as u8 * Self::MASK_EXEC)
				| (share as u8 * Self::MASK_SHARE),
		}
	}

	pub const fn read(&self) -> bool {
		self.bits & Self::MASK_READ != 0
	}

	pub const fn write(&self) -> bool {
		self.bits & Self::MASK_WRITE != 0
	}

	pub const fn exec(&self) -> bool {
		self.bits & Self::MASK_EXEC != 0
	}

	pub const fn shared(&self) -> bool {
		self.bits & Self::MASK_SHARE != 0
	}
}
impl std::ops::BitAnd<Self> for MemoryPagePermissions {
	type Output = Self;

	fn bitand(self, rhs: Self) -> Self::Output {
		MemoryPagePermissions {
			bits: self.bits & rhs.bits,
		}
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
	/// Like `File(path)` but the path is the original executable of the process.
	ProcessExecutable(PathBuf),
	/// File-backed mapping that is different from the process executable.
	File(PathBuf), // TODO: Research platforms more
	               // Deleted
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
			MemoryPageType::ProcessExecutable(path) => write!(f, "{} (self)", path.display()),
			MemoryPageType::File(path) => write!(f, "{}", path.display()),
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryPage {
	pub address_range: [OffsetType; 2],
	pub permissions: MemoryPagePermissions,
	pub offset: u64,
	pub page_type: MemoryPageType,
}
impl MemoryPage {
	pub fn try_merge_mut(&mut self, other: Self) -> Result<(), Self> {
		if self.address_range[1].get() < other.address_range[0].get()
			|| other.address_range[1].get() < self.address_range[0].get()
		{
			return Err(other);
		}

		self.address_range = [
			self.address_range[0].min(other.address_range[0]),
			self.address_range[1].max(other.address_range[1]),
		];
		self.permissions = self.permissions & other.permissions;
		self.offset = self.offset.min(other.offset);
		if self.page_type != other.page_type {
			self.page_type = MemoryPageType::Unknown;
		};

		Ok(())
	}

	/// Returns an adapted iterator that will merge all consecutive pages in the iterator using [`try_merge_mut`](MemoryPage::try_merge_mut).
	pub fn merge_sorted(iter: impl Iterator<Item = Self>) -> impl Iterator<Item = Self> {
		AccFilter::new(iter, |acc, curr| match acc {
			None => acc.replace(curr),
			Some(a) => match a.try_merge_mut(curr) {
				Ok(()) => None,
				Err(other) => acc.replace(other),
			},
		})
	}

	pub const fn start(&self) -> OffsetType {
		self.address_range[0]
	}

	pub const fn end(&self) -> OffsetType {
		self.address_range[1]
	}

	pub const fn size(&self) -> u64 {
		self.end().get() - self.start().get()
	}
}
impl std::fmt::Display for MemoryPage {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"{}-{} {} {} {}",
			self.address_range[0],
			self.address_range[1],
			self.permissions,
			self.offset,
			self.page_type
		)
	}
}

/// Trait for objects that serve as memory map storages.
///
/// The `containing_page` should only be implemented if the implementation can provide a more efficient search behavior.
pub trait MemoryMap {
	/// Returns an ordered slice of memory pages.
	fn pages(&self) -> &[MemoryPage];

	/// Returns the mapped memory page which contains the given offset.
	fn containing_page(&self, offset: OffsetType) -> Option<&MemoryPage> {
		self.pages()
			.iter()
			.find(|&p| offset >= p.address_range[0] && offset <= p.address_range[1])
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::OffsetType;

	use super::{MemoryPage, MemoryPagePermissions, MemoryPageType};

	#[test]
	fn test_memory_page_merge() {
		let mut left = MemoryPage {
			address_range: [OffsetType::new_unwrap(100), OffsetType::new_unwrap(200)],
			permissions: MemoryPagePermissions::new(true, true, false, true),
			offset: 0,
			page_type: MemoryPageType::Anon,
		};
		let right = MemoryPage {
			address_range: [OffsetType::new_unwrap(200), OffsetType::new_unwrap(300)],
			permissions: MemoryPagePermissions::new(true, false, true, false),
			offset: 100,
			page_type: MemoryPageType::Heap,
		};
		left.try_merge_mut(right).unwrap();

		assert_eq!(
			left,
			MemoryPage {
				address_range: [OffsetType::new_unwrap(100), OffsetType::new_unwrap(300)],
				permissions: MemoryPagePermissions::new(true, false, false, false),
				offset: 0,
				page_type: MemoryPageType::Unknown
			}
		);

		let mut left = MemoryPage {
			address_range: [OffsetType::new_unwrap(400), OffsetType::new_unwrap(500)],
			permissions: MemoryPagePermissions::new(true, true, false, true),
			offset: 400,
			page_type: MemoryPageType::Stack,
		};
		let right = MemoryPage {
			address_range: [OffsetType::new_unwrap(200), OffsetType::new_unwrap(400)],
			permissions: MemoryPagePermissions::new(true, false, true, false),
			offset: 200,
			page_type: MemoryPageType::Stack,
		};
		left.try_merge_mut(right).unwrap();

		assert_eq!(
			left,
			MemoryPage {
				address_range: [OffsetType::new_unwrap(200), OffsetType::new_unwrap(500)],
				permissions: MemoryPagePermissions::new(true, false, false, false),
				offset: 200,
				page_type: MemoryPageType::Stack
			}
		);
	}

	#[test]
	fn test_memory_page_merge_err() {
		let mut left = MemoryPage {
			address_range: [OffsetType::new_unwrap(400), OffsetType::new_unwrap(500)],
			permissions: MemoryPagePermissions::new(true, true, false, true),
			offset: 400,
			page_type: MemoryPageType::Stack,
		};
		let right = MemoryPage {
			address_range: [OffsetType::new_unwrap(200), OffsetType::new_unwrap(300)],
			permissions: MemoryPagePermissions::new(true, false, true, false),
			offset: 200,
			page_type: MemoryPageType::Stack,
		};
		left.try_merge_mut(right).unwrap_err();
	}
}
