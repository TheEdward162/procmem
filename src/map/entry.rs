use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Copy, Clone)]
pub struct MemoryMapPermissions {
	perms: u8
}
impl MemoryMapPermissions {
	const MASK_EXEC: u8 = 1 << 2;
	const MASK_READ: u8 = 1 << 0;
	const MASK_SHARE: u8 = 1 << 3;
	const MASK_WRITE: u8 = 1 << 1;

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
impl std::fmt::Display for MemoryMapPermissions {
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


#[derive(Debug, Clone)]
pub enum EntryType {
	Stack,
	Heap,
	Vvar,
	Vdso,
	Anon,
	File(PathBuf)
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


#[derive(Debug, Clone)]
pub struct MemoryMapEntry {
	/// Address range of this entry.
	pub address_range: [usize; 2],
	/// Permissions of this entry.
	pub permissions: MemoryMapPermissions,
	pub entry_type: EntryType
}
impl MemoryMapEntry {
	pub fn perms(&self) -> MemoryMapPermissions {
		self.permissions
	}
}
impl std::fmt::Display for MemoryMapEntry {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"{:x}-{:x} {} {}",
			self.address_range[0], self.address_range[1], self.permissions, self.entry_type
		)
	}
}


#[derive(Debug, Error)]
pub enum MemoryMapPermissionsParseError {
	#[error("invalid read permission: {0:?}")]
	InvalidRead(Option<char>),
	#[error("invalid write permission: {0:?}")]
	InvalidWrite(Option<char>),
	#[error("invalid exec permission: {0:?}")]
	InvalidExec(Option<char>),
	#[error("invalid share permission: {0:?}")]
	InvalidShare(Option<char>)
}
impl std::str::FromStr for MemoryMapPermissions {
	type Err = MemoryMapPermissionsParseError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let mut chars = s.trim().chars();

		let mut perms = 0;
		match chars.next() {
			Some('r') => {
				perms |= Self::MASK_READ;
			}
			Some('-') => (),
			ch => return Err(MemoryMapPermissionsParseError::InvalidRead(ch))
		}

		match chars.next() {
			Some('w') => {
				perms |= Self::MASK_WRITE;
			}
			Some('-') => (),
			ch => return Err(MemoryMapPermissionsParseError::InvalidWrite(ch))
		}

		match chars.next() {
			Some('x') => {
				perms |= Self::MASK_EXEC;
			}
			Some('-') => (),
			ch => return Err(MemoryMapPermissionsParseError::InvalidExec(ch))
		}

		match chars.next() {
			Some('s') => {
				perms |= Self::MASK_SHARE;
			}
			Some('p') => (),
			ch => return Err(MemoryMapPermissionsParseError::InvalidShare(ch))
		}

		Ok(MemoryMapPermissions { perms })
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

#[derive(Debug, Error)]
pub enum MemoryMapEntryParseError {
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
	ParseMapPerms(#[from] MemoryMapPermissionsParseError)
}
impl std::str::FromStr for MemoryMapEntry {
	type Err = MemoryMapEntryParseError;

	// <from>-<to> <perms> <offset> <dev> <inode> <path>
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let mut split = s.splitn(6, " ");

		let mut range_split = split
			.next()
			.ok_or(MemoryMapEntryParseError::InvalidRange)?
			.split('-');
		let from = usize::from_str_radix(
			range_split
				.next()
				.ok_or(MemoryMapEntryParseError::InvalidRange)?,
			16
		)?;
		let to = usize::from_str_radix(
			range_split
				.next()
				.ok_or(MemoryMapEntryParseError::InvalidRange)?,
			16
		)?;

		let permissions = split
			.next()
			.ok_or(MemoryMapEntryParseError::InvalidPerms)?
			.parse::<MemoryMapPermissions>()?;

		split
			.next()
			.ok_or(MemoryMapEntryParseError::InvalidDevnode)?;
		split.next().ok_or(MemoryMapEntryParseError::InvalidInode)?;
		split.next().ok_or(MemoryMapEntryParseError::InvalidInode)?;

		let entry_type = split
			.next()
			.ok_or(MemoryMapEntryParseError::InvalidEntry)?
			.parse::<EntryType>()
			.unwrap();

		Ok(MemoryMapEntry {
			address_range: [from, to],
			permissions,
			entry_type
		})
	}
}
