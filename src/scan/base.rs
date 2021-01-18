use std::{
	fs::{File, OpenOptions},
	io::{Read, Seek, SeekFrom}
};

use thiserror::Error;

use crate::{
	map::{MemoryMapEntry, MemoryPageIndex},
	process::{ProcessContext, PtraceAttachError}
};

use super::callback::ScanCallback;
use super::scanner::ByteScanner;
use super::{ScanFlow, ScanEntry};

#[derive(Debug, Error)]
pub enum ScanError {
	#[error("memory page with given index does not exist")]
	MissingMemoryPage,
	#[error(transparent)]
	PtraceAttachError(#[from] PtraceAttachError),
	#[error("could not read memory file")]
	Io(#[from] std::io::Error)
}

#[derive(Debug, Error)]
#[error("could not open read-only memory file")]
pub struct ScannerContextBaseError(#[from] std::io::Error);

pub struct ScannerContextBase {
	/// Readonly handle to the mem file.
	mem_ro: File
}
impl ScannerContextBase {
	pub fn new(process: &mut ProcessContext) -> Result<Self, ScannerContextBaseError> {
		let mem_path = ProcessContext::mem_path(process.pid());
		let mem_ro = OpenOptions::new().read(true).open(mem_path)?;

		Ok(ScannerContextBase { mem_ro })
	}

	/// ## Safety
	/// * `process` must be the same process that was used with `new`
	pub unsafe fn scan(
		&mut self,
		process: &mut ProcessContext,
		page: MemoryPageIndex,
		unaligned: bool,
		callback: impl ScanCallback
	) -> Result<(), ScanError> {
		process.ptrace_attach()?;

		let entry = process
			.memory_map()
			.page(page)
			.ok_or(ScanError::MissingMemoryPage)?;

		let result = self.scan_raw(entry, unaligned, callback);

		process.ptrace_detach()?;

		result
	}

	/// ## Safety
	// * ptrace must be attached on the process that was used with `new`
	pub unsafe fn scan_raw(
		&mut self,
		entry: &MemoryMapEntry,
		unaligned: bool,
		callback: impl ScanCallback
	) -> Result<(), ScanError> {
		// Seek to the page location
		self.mem_ro
			.seek(SeekFrom::Start(entry.address_range[0] as u64))?;

		// Scan the memory page
		Self::scan_data(
			&mut self.mem_ro,
			entry.address_range[0] .. entry.address_range[1],
			unaligned,
			callback
		)?;

		Ok(())
	}

	fn scan_data(
		mut data: impl Read,
		address_range: std::ops::Range<usize>,
		unaligned: bool,
		mut callback: impl ScanCallback
	) -> Result<(), ScanError> {
		let mut byte = [0u8; 1];
		let mut scanner = ByteScanner::new();

		for current_offset in address_range {
			data.read_exact(&mut byte)?;
			scanner.push(byte[0]);

			macro_rules! check_ready {
				(
					$ready_fn: ident;
					$(
						$local_type: ident
					),+
				) => {
					$(
						if scanner.$ready_fn::<$local_type>() {
							let flow = callback.handle(
								ScanEntry::$local_type(
									current_offset + 1 - std::mem::size_of::<$local_type>(),
									scanner.read::<$local_type>()
								)
							);

							if flow == ScanFlow::Break {
								break;
							}
						}
					)+
				};
			}

			if unaligned {
				check_ready!(
					ready_unaligned;
					u64, f64, u32, f32, u16, u8
				);
			} else {
				check_ready!(
					ready;
					u64, f64, u32, f32, u16, u8
				);
			}
		}

		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::scan::callback::ScanCallbackClosure;

    use super::{super::{ScanEntry, ScanFlow}, ScannerContextBase};

	#[test]
	fn test_scanner_context_base() {
		let data: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

		let mut entries = Vec::<ScanEntry>::new();
		ScannerContextBase::scan_data(
			data.as_ref(), 
			0 .. data.len(), 
			false, 
			ScanCallbackClosure::from(
				|entry| {
					entries.push(entry);
					ScanFlow::Continue
				}
			)
		).unwrap();

		macro_rules! assert_contains_all {
			(
				$(
					$ex: expr
				),+ $(,)?
			) => {
				$(
					let entry = entries.iter().find(
						|&e| e == &$ex
					);
					assert!(entry.is_some());
				)+
			}
		}

		dbg!(&entries);
		assert_contains_all!(
			ScanEntry::u64(
				0,
				0 + (1 << 8)
					+ (2 << 16) + (3 << 24)
					+ (4 << 32) + (5 << 40)
					+ (6 << 48) + (7 << 56)
			),
			ScanEntry::f64(0, f64::from_ne_bytes(data)),
			ScanEntry::u32(0, 0 + (1 << 8) + (2 << 16) + (3 << 24)),
			ScanEntry::f32(0, f32::from_ne_bytes([data[0], data[1], data[2], data[3]])),
			ScanEntry::u32(4, 4 + (5 << 8) + (6 << 16) + (7 << 24)),
			ScanEntry::f32(4, f32::from_ne_bytes([data[4], data[5], data[6], data[7]])),
			ScanEntry::u16(0, 0 + (1 << 8)),
			ScanEntry::u16(2, 2 + (3 << 8)),
			ScanEntry::u16(4, 4 + (5 << 8)),
			ScanEntry::u16(6, 6 + (7 << 8)),
			ScanEntry::u8(0, 0),
			ScanEntry::u8(1, 1),
			ScanEntry::u8(2, 2),
			ScanEntry::u8(3, 3),
			ScanEntry::u8(4, 4),
			ScanEntry::u8(5, 5),
			ScanEntry::u8(6, 6),
			ScanEntry::u8(7, 7),
		);
	}
}
