use std::{collections::HashMap, fs::OpenOptions, io::Read, path::Path};

use thiserror::Error;

pub mod entry;
pub use entry::*;

#[derive(Debug, Error)]
pub enum LoadMapError {
	#[error("could not read map file")]
	Io(#[from] std::io::Error),
	#[error("could not parse map")]
	Parse(#[from] MemoryMapEntryParseError)
}

pub type MemoryPageIndex = crate::util::OffsetType;

pub struct MemoryMap {
	entries: HashMap<MemoryPageIndex, MemoryMapEntry>
}
impl MemoryMap {
	/// Loads memory map info from `/proc/$PID/maps` formatted file.
	pub fn load(path: impl AsRef<Path>) -> Result<Self, LoadMapError> {
		let mut entries = HashMap::new();

		let mut file = OpenOptions::new().read(true).open(path)?;
		let mut buffer = String::new();
		file.read_to_string(&mut buffer)?;

		for line in buffer.lines() {
			let entry: MemoryMapEntry = line.parse()?;
			entries.insert(entry.address_range[0].into(), entry);
		}

		Ok(MemoryMap { entries })
	}

	pub fn page(&self, index: MemoryPageIndex) -> Option<&MemoryMapEntry> {
		self.entries.get(&index)
	}

	pub fn values(&self) -> impl Iterator<Item = &MemoryMapEntry> {
		self.entries.values()
	}
}
