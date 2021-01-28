use std::{
	fs::{File, OpenOptions},
	io::{Read, Seek, SeekFrom, Write}
};

use thiserror::Error;

use crate::{
	common::OffsetType,
	memory::access::{
		ExclusiveLockError,
		LockError,
		MemoryAccess,
		ReadError,
		UnlockError,
		WriteError
	}
};

#[derive(Debug, Error)]
pub enum ProcfsAccessOpenError {
	#[error("could not open memory file")]
	MemoryIo(std::io::Error),
	#[error("ptrace(PTRACE_ATTACH) failed")]
	PtraceError(std::io::Error),
	#[error("waitpid failed")]
	WaitpidError(std::io::Error)
}

/// Procfs implementation of memory access.
///
/// Uses `ptrace` to lock (stop) the process. Ptrace is attached only the first time a lock is acquired, not when the process is opened.
///
/// Ptrace is detached on drop.
pub struct ProcfsAccess {
	pid: libc::pid_t,
	ptrace_attached: bool,
	ptrace_lock: usize,
	mem: File
}
impl ProcfsAccess {
	pub fn mem_path(pid: libc::pid_t) -> std::path::PathBuf {
		format!("/proc/{}/mem", pid).into()
	}

	/// Opens a process with given `pid`.
	///
	/// The process memory access file is located in `/proc/[pid]/mem`.
	pub fn open(pid: libc::pid_t) -> Result<Self, ProcfsAccessOpenError> {
		let path = Self::mem_path(pid);

		let mem = OpenOptions::new()
			.read(true)
			.write(true)
			.open(path)
			.map_err(|err| ProcfsAccessOpenError::MemoryIo(err))?;

		Ok(ProcfsAccess {
			pid,
			ptrace_attached: false,
			ptrace_lock: 0,
			mem
		})
	}

	unsafe fn ptrace_attach(&mut self) -> Result<(), LockError> {
		debug_assert!(!self.ptrace_attached);
		
		if libc::ptrace(libc::PTRACE_ATTACH, self.pid, 0, 0) != 0 {
			return Err(LockError::PtraceError(std::io::Error::last_os_error()))
		}

		// wait until the signal is delivered
		let waitpid_res = libc::waitpid(self.pid, std::ptr::null_mut(), 0);
		if waitpid_res == -1 {
			return Err(LockError::WaitpidError(std::io::Error::last_os_error()))
		}
		debug_assert_eq!(waitpid_res, self.pid);

		self.ptrace_attached = true;

		Ok(())
	}

	unsafe fn ptrace_stop(&mut self) -> Result<(), LockError> {
		if libc::kill(self.pid, libc::SIGSTOP) != 0 {
			return Err(LockError::SigstopError(std::io::Error::last_os_error()))
		}

		// wait until the signal is delivered
		// TODO: read the manpage
		let waitpid_res = libc::waitpid(self.pid, std::ptr::null_mut(), 0);
		if waitpid_res == -1 {
			return Err(LockError::WaitpidError(std::io::Error::last_os_error()))
		}
		debug_assert_eq!(waitpid_res, self.pid);

		Ok(())
	}

	unsafe fn ptrace_cont(&mut self) -> Result<(), UnlockError> {
		if libc::ptrace(libc::PTRACE_CONT, self.pid, 0, 0) != 0 {
			return Err(UnlockError::PtraceError(std::io::Error::last_os_error()))
		}

		Ok(())
	}

	unsafe fn ptrace_detach(&mut self) -> Result<(), UnlockError> {
		debug_assert!(self.ptrace_attached);

		if libc::ptrace(libc::PTRACE_DETACH, self.pid, 0, 0) != 0 {
			return Err(UnlockError::PtraceError(std::io::Error::last_os_error()))
		}
		self.ptrace_attached = false;

		Ok(())
	}
}
impl MemoryAccess for ProcfsAccess {
	fn lock(&mut self) -> Result<bool, LockError> {
		let result = if !self.ptrace_attached {
			unsafe {
				self.ptrace_attach()?;
			}

			true
		} else if self.ptrace_lock == 0 {
			unsafe {
				self.ptrace_stop()?;
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
			self.lock()?;
		} else {
			return Err(ExclusiveLockError::AlreadyLocked)
		}

		Ok(())
	}

	fn unlock(&mut self) -> Result<bool, UnlockError> {
		if self.ptrace_lock == 0 {
			return Err(UnlockError::NotLocked)
		}

		self.ptrace_lock -= 1;
		if self.ptrace_lock == 0 {
			unsafe {
				self.ptrace_cont()?;
			}

			Ok(true)
		} else {
			Ok(false)
		}
	}

	unsafe fn read(&mut self, offset: OffsetType, buffer: &mut [u8]) -> Result<(), ReadError> {
		self.mem.seek(SeekFrom::Start(offset.get() as u64))?;

		self.mem.read_exact(buffer)?;

		Ok(())
	}

	unsafe fn write(&mut self, offset: OffsetType, data: &[u8]) -> Result<(), WriteError> {
		self.mem.seek(SeekFrom::Start(offset.get() as u64))?;

		self.mem.write_all(data)?;

		Ok(())
	}
}
impl Drop for ProcfsAccess {
	fn drop(&mut self) {
		if self.ptrace_attached {
			if self.ptrace_lock == 0 {
				// need to stop the process to detach from it, weirdly
				unsafe {
					self.ptrace_stop().unwrap();
				}
			}

			unsafe {
				self.ptrace_detach().unwrap();
			}
		}
	}
}
