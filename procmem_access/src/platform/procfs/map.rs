use std::{
	fs::{self, OpenOptions},
	io::Read
};

use thiserror::Error;

use crate::{
	common::OffsetType,
	memory::map::{MemoryMap, MemoryPage, MemoryPagePermissions, MemoryPageType}
};

#[derive(Debug, Error)]
pub enum ProcfsMemoryMapLoadError {
	#[error("could not read map file")]
	Io(#[from] std::io::Error),
	#[error(transparent)]
	MemoryPageParseError(#[from] MemoryPageParseError)
}

pub struct ProcfsMemoryMap {
	#[allow(dead_code)]
	pid: libc::pid_t,
	pages: Vec<MemoryPage>
}
impl ProcfsMemoryMap {
	fn map_path(pid: libc::pid_t) -> std::path::PathBuf {
		format!("/proc/{}/maps", pid).into()
	}

	pub fn new(pid: libc::pid_t) -> Result<Self, ProcfsMemoryMapLoadError> {
		let path = Self::map_path(pid);

		let mut pages = Vec::new();

		let mut file = OpenOptions::new().read(true).open(path)?;
		let mut buffer = String::new();
		// TODO: Lets hope there not invalid unicode in the file paths
		file.read_to_string(&mut buffer)?;

		let exe_path = fs::read_link(format!("/proc/{}/exe", pid))
			.ok()
			.and_then(|p| p.into_os_string().into_string().ok());

		for line in buffer.lines() {
			let page = Self::parse_map_line(line, exe_path.as_deref())?;

			pages.push(page);
		}

		Ok(ProcfsMemoryMap {
			pid,
			pages
		})
	}

	fn parse_page_permissions(
		string: &str
	) -> Result<MemoryPagePermissions, MemoryPagePermissionsParseError> {
		let mut chars = string.trim().chars();

		let read = match chars.next() {
			Some('r') => true,
			Some('-') => false,
			ch => return Err(MemoryPagePermissionsParseError::InvalidRead(ch))
		};

		let write = match chars.next() {
			Some('w') => true,
			Some('-') => false,
			ch => return Err(MemoryPagePermissionsParseError::InvalidWrite(ch))
		};

		let exec = match chars.next() {
			Some('x') => true,
			Some('-') => false,
			ch => return Err(MemoryPagePermissionsParseError::InvalidExec(ch))
		};

		let share = match chars.next() {
			Some('s') => true,
			Some('p') => false,
			ch => return Err(MemoryPagePermissionsParseError::InvalidShare(ch))
		};

		Ok(MemoryPagePermissions::new(read, write, exec, share))
	}

	fn parse_page_type(string: &str, exe_path: Option<&str>) -> MemoryPageType {
		match string.trim() {
			"[stack]" => MemoryPageType::Stack,
			"[heap]" => MemoryPageType::Heap,
			"" => MemoryPageType::Anon,

			// [vvar] [vdso]
			s if s.starts_with('[') && s.ends_with(']') => MemoryPageType::Unknown,
			s if s.ends_with("(deleted)") => MemoryPageType::Unknown,

			path => match exe_path {
				Some(exe) if path == exe => {
					MemoryPageType::ProcessExecutable(std::path::PathBuf::from(path))
				}
				_ => MemoryPageType::File(std::path::PathBuf::from(path))
			}
		}
	}

	fn parse_map_line(
		line: &str,
		exe_path: Option<&str>
	) -> Result<MemoryPage, MemoryPageParseError> {
		let mut split = line.splitn(6, " ");

		let mut range_split = split
			.next()
			.ok_or(MemoryPageParseError::InvalidRange)?
			.split('-');
		let from = u64::from_str_radix(
			range_split
				.next()
				.ok_or(MemoryPageParseError::InvalidRange)?,
			16
		)?;
		let to = u64::from_str_radix(
			range_split
				.next()
				.ok_or(MemoryPageParseError::InvalidRange)?,
			16
		)?;

		let permissions =
			Self::parse_page_permissions(split.next().ok_or(MemoryPageParseError::InvalidPerms)?)?;

		split.next().ok_or(MemoryPageParseError::InvalidDevnode)?;
		split.next().ok_or(MemoryPageParseError::InvalidInode)?;
		let offset = split
			.next()
			.ok_or(MemoryPageParseError::InvalidOffset)?
			.parse::<u64>()?;

		let page_type = Self::parse_page_type(
			split.next().ok_or(MemoryPageParseError::InvalidEntry)?,
			exe_path
		);

		Ok(MemoryPage {
			address_range: [OffsetType::new_unwrap(from), OffsetType::new_unwrap(to)],
			permissions,
			offset,
			page_type
		})
	}
}
impl MemoryMap for ProcfsMemoryMap {
	fn pages(&self) -> &[MemoryPage] {
		&self.pages
	}
}

#[derive(Debug, Error)]
pub enum MemoryPagePermissionsParseError {
	#[error("invalid read permission: {0:?}")]
	InvalidRead(Option<char>),
	#[error("invalid write permission: {0:?}")]
	InvalidWrite(Option<char>),
	#[error("invalid exec permission: {0:?}")]
	InvalidExec(Option<char>),
	#[error("invalid share permission: {0:?}")]
	InvalidShare(Option<char>)
}
#[derive(Debug, Error)]
pub enum MemoryPageParseError {
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
	ParseMapPerms(#[from] MemoryPagePermissionsParseError)
}

#[cfg(test)]
mod test {
	use super::ProcfsMemoryMap;
	use crate::{memory::map::{MemoryPage, MemoryPagePermissions, MemoryPageType}, prelude::OffsetType};

	#[test]
	fn test_procfs_maps_parse() {
		let line = "1f0-20f rw-p 0 00:00 0 [heap]";

		let value = ProcfsMemoryMap::parse_map_line(line, None).unwrap();
		assert_eq!(
			value,
			MemoryPage {
				address_range: [OffsetType::new_unwrap(496), OffsetType::new_unwrap(527)],
				permissions: MemoryPagePermissions::new(true, true, false, false),
				offset: 0,
				page_type: MemoryPageType::Heap
			}
		);
	}
}
