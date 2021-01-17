use std::path::{PathBuf, Path};
use std::fs::OpenOptions;
use std::io::Read;

use thiserror::Error;

#[derive(Debug, Copy, Clone)]
pub struct MapPerms {
	perms: u8
}
impl MapPerms {
	const MASK_READ: u8 = 1 << 0;
	const MASK_WRITE: u8 = 1 << 1;
	const MASK_EXEC: u8 = 1 << 2;
	const MASK_SHARE: u8 = 1 << 3;

	pub fn read(&self) -> bool {
		self.perms & Self::MASK_READ != 0
	}

	pub fn write(&self) -> bool {
		self.perms & Self::MASK_WRITE != 0
	}

	pub fn exec(&self) -> bool {
		self.perms & Self::MASK_EXEC != 0
	}

	pub fn shared(&self) -> bool {
		self.perms & Self::MASK_SHARE != 0
	}
}
impl std::fmt::Display for MapPerms {
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

#[derive(Debug, Error)]
pub enum MapPermsParseError {
	#[error("invalid read permission: {0:?}")]
	InvalidRead(Option<char>),
	#[error("invalid write permission: {0:?}")]
	InvalidWrite(Option<char>),
	#[error("invalid exec permission: {0:?}")]
	InvalidExec(Option<char>),
	#[error("invalid share permission: {0:?}")]
	InvalidShare(Option<char>)
}
impl std::str::FromStr for MapPerms {
	type Err = MapPermsParseError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let mut chars = s.trim().chars();
		
		let mut perms = 0;
		match chars.next() {
			Some('r') => { perms |= Self::MASK_READ; },
			Some('-') => (),
			ch => return Err(MapPermsParseError::InvalidRead(ch))
		}

		match chars.next() {
			Some('w') => { perms |= Self::MASK_WRITE; },
			Some('-') => (),
			ch => return Err(MapPermsParseError::InvalidWrite(ch))
		}

		match chars.next() {
			Some('x') => { perms |= Self::MASK_EXEC; },
			Some('-') => (),
			ch => return Err(MapPermsParseError::InvalidExec(ch))
		}

		match chars.next() {
			Some('s') => { perms |= Self::MASK_SHARE; },
			Some('p') => (),
			ch => return Err(MapPermsParseError::InvalidShare(ch))
		}

		Ok(
			MapPerms {
				perms
			}
		)
	}
}

#[derive(Debug, Clone)]
pub enum EntryType {
	Stack,
	Heap,
	Vvar,
	Vdso,
	Anon,
	File(PathBuf),
}
impl std::fmt::Display for EntryType {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			EntryType::Stack => write!(f, "[stack]"),
			EntryType::Heap => write!(f, "[heap]"),
			EntryType::Vvar => write!(f, "[vvar]"),
			EntryType::Vdso => write!(f, "[vdso]"),
			EntryType::Anon => write!(f, ""),
			EntryType::File(path) => write!(f, "{}", path.display())
		}
	}
}
impl std::str::FromStr for EntryType {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let v = match s.trim() {
			"[stack]" => EntryType::Stack,
			"[heap]" => EntryType::Heap,
			"[vvar]" => EntryType::Vvar,
			"[vdso]" => EntryType::Vdso,
			"" => EntryType::Anon,
			p => EntryType::File(PathBuf::from_str(p)?)
		};

		Ok(v)
	}
}

#[derive(Debug, Clone)]
pub struct MapEntry {
	pub range: [usize; 2],
	pub perms: MapPerms,
	pub entry_type: EntryType
}
impl MapEntry {
	pub fn perms(&self) -> MapPerms {
		self.perms
	}
}
impl std::fmt::Display for MapEntry {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"{:x}-{:x} {} {}",
			self.range[0], self.range[1],
			self.perms,
			self.entry_type
		)
	}
}

#[derive(Debug, Error)]
pub enum MapEntryParseError {
	#[error("mapped range has invalid format")]
	InvalidRange,
	#[error("permissions have invalid format")]
	InvalidPerms,
	#[error("offset has invalid format")]
	InvalidOffset,
	#[error("devnode has invalid format")]
	InvalidDevnode,
	#[error("inode has invalid format")]
	InvalidInode,
	#[error("entry type has invalid format")]
	InvalidEntry,

	#[error("could not parse range bounds")]
	ParseUsize(#[from] std::num::ParseIntError),
	#[error("could not parse map permissions")]
	ParseMapPerms(#[from] MapPermsParseError)
}
impl std::str::FromStr for MapEntry {
	type Err = MapEntryParseError;

	// <from>-<to> <perms> <offset> <dev> <inode> <path>
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let mut split = s.splitn(6, " ");

		let mut range_split = split.next().ok_or(MapEntryParseError::InvalidRange)?.split('-');
		let from = usize::from_str_radix(range_split.next().ok_or(MapEntryParseError::InvalidRange)?, 16)?;
		let to = usize::from_str_radix(range_split.next().ok_or(MapEntryParseError::InvalidRange)?, 16)?;

		let perms = split.next().ok_or(MapEntryParseError::InvalidPerms)?.parse::<MapPerms>()?;

		split.next().ok_or(MapEntryParseError::InvalidDevnode)?;
		split.next().ok_or(MapEntryParseError::InvalidInode)?;
		split.next().ok_or(MapEntryParseError::InvalidInode)?;

		let entry_type = split.next().ok_or(MapEntryParseError::InvalidEntry)?.parse::<EntryType>().unwrap();
		
		Ok(
			MapEntry {
				range: [from, to],
				perms,
				entry_type
			}
		)
	}
}

#[derive(Debug, Error)]
pub enum LoadMapError {
	#[error("file io error")]
	Io(#[from] std::io::Error),
	#[error("map parse error")]
	Parse(#[from] MapEntryParseError)
} 
/// Loads memory map info from `/proc/$PID/maps` formatted file.
pub fn load_maps(path: impl AsRef<Path>) -> Result<Vec<MapEntry>, LoadMapError> {
	let mut entries = Vec::new();

	let mut file = OpenOptions::new().read(true).open(path)?;
	let mut buffer = String::new();
	file.read_to_string(&mut buffer)?;
	
	for line in buffer.lines() {
		let entry: MapEntry = line.parse()?;
		entries.push(entry);
	}

	Ok(entries)
}