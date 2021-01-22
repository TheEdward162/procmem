use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};

use thiserror::Error;

use crate::common::OffsetType;
use crate::memory::access::{
	LockError,
	ExclusiveLockError,
	UnlockError,
	ReadError,
	WriteError,
	MemoryAccess
};

#[derive(Debug, Error)]
pub enum ProcfsAccessOpenError {
	#[error("could not open memory file")]
	MemoryIo(#[from] std::io::Error),
}

pub struct ProcfsAccess {
	pid: libc::pid_t,
	ptrace_lock: usize,
	mem: File
}
impl ProcfsAccess {
	pub fn mem_path(pid: libc::pid_t) -> std::path::PathBuf {
		format!("/proc/{}/mem", pid).into()
	}

	pub fn open(pid: libc::pid_t) -> Result<Self, ProcfsAccessOpenError> {
		let path = Self::mem_path(pid);
		
		let mem = OpenOptions::new().read(true).write(true).open(path).map_err(
			|err| ProcfsAccessOpenError::MemoryIo(err)
		)?;

		Ok(
			ProcfsAccess {
				pid,
				ptrace_lock: 0,
				mem
			}
		)
	}

	/// ## Safety
	/// * ?? maybe it doesn't need to be unsafe?
	unsafe fn ptrace_attach(&mut self) -> Result<(), LockError> {
		if libc::ptrace(libc::PTRACE_ATTACH, self.pid, 0, 0) != 0 {
			return Err(
				LockError::PtraceError(std::io::Error::last_os_error())
			)
		}

		let waitpid_res = libc::waitpid(self.pid, std::ptr::null_mut(), 0);
		if waitpid_res == -1 {
			return Err(
				LockError::WaitpidError(std::io::Error::last_os_error())
			)
		}
		debug_assert_eq!(waitpid_res, self.pid);

		Ok(())
	}

	/// ## Safety
	/// * ?? maybe it doesn't need to be unsafe?
	unsafe fn ptrace_detach(&mut self) -> Result<(), UnlockError> {
		if libc::ptrace(libc::PTRACE_DETACH, self.pid, 0, 0) != 0 {
			return Err(
				UnlockError::PtraceError(std::io::Error::last_os_error())
			)
		}

		Ok(())
	}
}
impl MemoryAccess for ProcfsAccess {
	fn lock(&mut self) -> Result<bool, LockError> {
		let result = if self.ptrace_lock == 0 {
			unsafe {
				self.ptrace_attach()?;
			}

			true
		} else {
			false
		};

		self.ptrace_lock += 1;
		Ok(result)
	}

	fn lock_exlusive(&mut self) -> Result<(), ExclusiveLockError> {
		if self.ptrace_lock == 0 {
			unsafe {
				self.ptrace_attach()?;
			}
		} else {
			return Err(ExclusiveLockError::AlreadyLocked);
		}

		Ok(())
	}

	fn unlock(&mut self) -> Result<bool, UnlockError> {
		if self.ptrace_lock == 0 {
			return Err(UnlockError::NotLocked);
		}

		self.ptrace_lock -= 1;
		if self.ptrace_lock == 0 {
			unsafe {
				self.ptrace_detach()?;
			}

			Ok(true)
		} else {
			Ok(false)
		}
	}

	unsafe fn read(&mut self, offset: OffsetType, buffer: &mut [u8]) -> Result<(), ReadError> {
		self.mem.seek(
			SeekFrom::Start(offset.get()as u64)
		)?;

		self.mem.read_exact(buffer)?;

		Ok(())
	}

	unsafe fn write(&mut self, offset: OffsetType, data: &[u8]) -> Result<(), WriteError> {
		self.mem.seek(
			SeekFrom::Start(offset.get() as u64)
		)?;

		self.mem.write_all(data)?;

		Ok(())
	}
}
impl Drop for ProcfsAccess {
	fn drop(&mut self) {
		if self.ptrace_lock > 0 {
			unsafe {
				self.ptrace_detach().unwrap()
			}
		}
	}
}