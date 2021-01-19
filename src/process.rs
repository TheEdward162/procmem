use std::{
	fs::{File, OpenOptions},
	io::{Read, Seek, SeekFrom, Write},
	path::PathBuf
};

use thiserror::Error;

use crate::{
	map::{LoadMapError, MemoryMap},
	util::OffsetType
};

#[derive(Debug, Error)]
pub enum ProcessContextError {
	#[error(transparent)]
	LoadMapError(#[from] LoadMapError),
	#[error("could not open memory file")]
	MemoryFileIo(std::io::Error)
}
#[derive(Debug, Error)]
pub enum PtraceAttachError {
	#[error("waitpid error {0}")]
	WaitpidError(std::io::Error),
	#[error("ptrace not attached")]
	NotAttached,
	#[error("cannot attach exclusively since the ptrace is already attached")]
	AlreadyAttached
}
#[derive(Debug, Error)]
pub enum ReadMemoryError {
	#[error(transparent)]
	PtraceAttachError(#[from] PtraceAttachError),

	#[error("could not read from memory file")]
	Io(#[from] std::io::Error)
}
#[derive(Debug, Error)]
pub enum WriteMemoryError {
	#[error(transparent)]
	PtraceAttachError(#[from] PtraceAttachError),

	#[error("could not write to memory file")]
	Io(#[from] std::io::Error) /* TODO
	                            * #[error("attempted to write to memory range that is not (wholly) mapped")]
	                            * RangeNotMapped */
}


/// Context of one process.
///
/// Handles attaching and detaching ptraces, loading mappings and has readwrite access to the memory.
pub struct ProcessContext {
	pid: libc::pid_t,
	memory_map: MemoryMap,

	/// Number of times ptrace attach was requested.
	///
	/// Ptrace attach is not released until this number reaches 0 again.
	ptrace_attach: usize,

	/// The only readwrite copy of the file
	mem_rw: File
}
impl ProcessContext {
	pub fn new(pid: libc::pid_t) -> Result<Self, ProcessContextError> {
		let map_path = Self::maps_path(pid);
		let memory_map = MemoryMap::load(map_path)?;

		let mem_path = Self::mem_path(pid);
		let mem_rw = OpenOptions::new()
			.read(true)
			.write(true)
			.open(mem_path)
			.map_err(|err| ProcessContextError::MemoryFileIo(err))?;

		Ok(ProcessContext {
			pid,
			memory_map,
			ptrace_attach: 0,
			mem_rw
		})
	}

	/// Attaches ptrace to the current process.
	///
	/// Returns false if ptrace is already attached (but still increases the internal counter).
	pub fn ptrace_attach(&mut self) -> Result<bool, PtraceAttachError> {
		let mut result = false;

		if self.ptrace_attach == 1 {
			unsafe {
				self.ptrace_attach_raw()?;
			}
			result = true;
		}
		self.ptrace_attach += 1;

		Ok(result)
	}

	pub fn ptrace_attach_exclusive(&mut self) -> Result<(), PtraceAttachError> {
		if self.ptrace_attach > 0 {
			return Err(PtraceAttachError::AlreadyAttached)
		}

		self.ptrace_attach()?;

		Ok(())
	}

	/// Detaches ptrace from the current process.
	///
	/// Returns false if ptrace is still attached (but still decreases the internal counter).
	pub fn ptrace_detach(&mut self) -> Result<bool, PtraceAttachError> {
		if self.ptrace_attach == 0 {
			return Err(PtraceAttachError::NotAttached)
		}

		let mut result = false;

		self.ptrace_attach -= 1;
		if self.ptrace_attach == 0 {
			unsafe { self.ptrace_detach_raw()? }
			result = true;
		}

		Ok(result)
	}

	/// ## Safety
	/// * TODO: Manpages are evil
	unsafe fn ptrace_attach_raw(&mut self) -> Result<(), PtraceAttachError> {
		libc::ptrace(libc::PTRACE_ATTACH, self.pid, 0, 0);

		if libc::waitpid(self.pid, std::ptr::null_mut(), 0) != 0 {
			return Err(PtraceAttachError::WaitpidError(
				std::io::Error::last_os_error()
			))
		}

		Ok(())
	}

	/// ## Safety
	/// * TODO: Manpages are evil
	unsafe fn ptrace_detach_raw(&mut self) -> Result<(), PtraceAttachError> {
		libc::ptrace(libc::PTRACE_DETACH, self.pid, 0, 0);

		Ok(())
	}

	/// Safety
	/// * read range must be mapped
	pub unsafe fn read_memory(
		&mut self,
		offset: OffsetType,
		buffer: &mut [u8]
	) -> Result<(), ReadMemoryError> {
		self.ptrace_attach()?;

		// TODO: Check memory range is mapped
		self.mem_rw.seek(SeekFrom::Start(offset.get() as u64))?;
		self.mem_rw.read_exact(buffer)?;

		self.ptrace_detach()?;

		Ok(())
	}

	/// Safety
	/// * written range must be mapped
	pub unsafe fn write_memory(
		&mut self,
		offset: OffsetType,
		data: &[u8]
	) -> Result<(), WriteMemoryError> {
		self.ptrace_attach_exclusive()?;

		// TODO: Check memory range is mapped
		self.mem_rw.seek(SeekFrom::Start(offset.get() as u64))?;
		self.mem_rw.write_all(data)?;

		self.ptrace_detach()?;

		Ok(())
	}

	pub fn pid(&self) -> libc::pid_t {
		self.pid
	}

	pub fn memory_map(&self) -> &MemoryMap {
		&self.memory_map
	}

	pub fn maps_path(pid: libc::pid_t) -> PathBuf {
		PathBuf::from(format!("/proc/{}/maps", pid))
	}

	pub fn mem_path(pid: libc::pid_t) -> PathBuf {
		PathBuf::from(format!("/proc/{}/mem", pid))
	}
}
impl Drop for ProcessContext {
	fn drop(&mut self) {
		if self.ptrace_attach > 0 {
			unsafe {
				self.ptrace_detach_raw()
					.expect("could not detach ptrace on drop")
			}
		}
	}
}
