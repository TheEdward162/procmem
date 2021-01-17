use std::fs::{OpenOptions, File};
use std::sync::{Mutex, Arc};
use std::io::{Seek, Read, SeekFrom};

use crate::map::MapEntry;

use super::ProcessContext;

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone)]
pub enum ScanEntryData {
	u8(u8),
	u16(u16),
	u32(u32),
	u64(u64),
	usize(usize),
	f32(f32),
	f64(f64),
}
#[derive(Debug)]
pub struct ScanEntry {
	/// Offset in the memory - basically the pointer in the process memory space.
	pub offset: usize,
	/// Data at the scanned offset.
	pub data: ScanEntryData
}
#[derive(Debug, Copy, Clone)]
pub enum ScanFlow {
	Continue,
	Break
}

/// Context of a scanner of a process.
///
/// Is attached to process context and has read-only access to the memory.
pub struct ScannerContext {
	process: Arc<Mutex<ProcessContext>>,
	/// Readonly handle to the mem file.
	mem_ro: File,
}
impl ScannerContext {
	pub fn scan(
		&mut self,
		page: usize,
		mut callback: impl FnMut(ScanEntry) -> ScanFlow
	) {
		let map_entry: MapEntry = {
			let parent_lock = self.process.lock().unwrap();
			parent_lock.maps().get(page).expect("TODO").clone()
		};
		self.mem_ro.seek(
			SeekFrom::Start(map_entry.range[0] as u64)
		).expect("TODO");

		for current_offset in map_entry.range[0] .. map_entry.range[1] {
			let res = Self::scan_data(
				&mut self.mem_ro,
				current_offset,
				map_entry.range[1],
				&mut callback
			).expect("TODO");

			if let ScanFlow::Break = res {
				return;
			}
		}

		// Ok(())
	}

	fn scan_data(
		mut stream: impl Read,
		current_offset: usize,
		end_offset: usize,
		mut callback: impl FnMut(ScanEntry) -> ScanFlow
	) -> Result<ScanFlow, std::io::Error> {
		let remaining_bytes = end_offset - current_offset;
		
		let mut buffer = [0u8; std::mem::size_of::<u64>()];

		macro_rules! generate_scan_result {
			(
				$size_ty: ty;
				$(
					$local_type: ident + $local_offset: literal;
				)+
			) => {
				if remaining_bytes >= std::mem::size_of::<$size_ty>() {
					stream.read_exact(&mut buffer[.. std::mem::size_of::<$size_ty>()])?;

					$(
						let res = callback(
							ScanEntry {
								offset: current_offset + $local_offset,
								data: ScanEntryData::$local_type(
									unsafe {
										std::ptr::read(
											(buffer.as_ptr() as *const $local_type).add($local_offset)
										)
									}
								)
							}
						);
						if let ScanFlow::Break = res {
							return Ok(ScanFlow::Break)
						}
					)+

					return Ok(ScanFlow::Continue)
				}
			}
		}

		generate_scan_result!(
			u64;
			u64 + 0; f64 + 0;
			u32 + 0; u32 + 1; f32 + 0; f32 + 1;
			u16 + 0; u16 + 1; u16 + 2; u16 + 3;
			u8 + 0; u8 + 1; u8 + 2; u8 + 3; u8 + 4; u8 + 5; u8 + 6; u8 + 7;
		);
		generate_scan_result!(
			u32;
			u32 + 0; u32 + 1;
			u16 + 0; u16 + 1;
			u8 + 0; u8 + 1; u8 + 2; u8 + 3;
		);
		generate_scan_result!(
			u16;
			u16 + 0;
			u8 + 0; u8 + 1;
		);
		generate_scan_result!(
			u8;
			u8 + 0;
		);

		Ok(ScanFlow::Continue)
	}
}