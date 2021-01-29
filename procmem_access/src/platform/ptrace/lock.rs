use thiserror::Error;

use crate::{
	memory::lock::{
		ExclusiveLockError,
		LockError,
		MemoryLock,
		UnlockError
	}
};

#[derive(Debug, Error)]
pub enum PtraceLockError {
	#[error("ptrace attach failed")]
	PtraceAttach(std::io::Error),
	#[error("kill(SIGSTOP) failed")]
	SigstopError(std::io::Error),
	#[error("waitpid failed")]
	WaitpidError(std::io::Error),
	#[error("ptrace continue failed")]
	PtraceCont(std::io::Error),
	#[error("ptrace detach failed")]
	PtraceDetach(std::io::Error)
}
impl From<PtraceLockError> for LockError {
	fn from(err: PtraceLockError) -> Self {
		LockError::PlatformError(Box::new(err))
	}
}
impl From<PtraceLockError> for UnlockError {
	fn from(err: PtraceLockError) -> Self {
		UnlockError::PlatformError(Box::new(err))
	}
}

pub struct PtraceLock {
	pid: libc::pid_t,
	ptrace_attached: bool,
	ptrace_lock: usize,
}
impl PtraceLock {
	pub fn new(pid: libc::pid_t) -> Self {
		PtraceLock {
			pid,
			ptrace_attached: false,
			ptrace_lock: 0
		}
	}

	unsafe fn ptrace_attach(&mut self) -> Result<(), PtraceLockError> {
		debug_assert!(!self.ptrace_attached);

		#[cfg(target_os = "linux")]
		let ptrace_res = libc::ptrace(libc::PTRACE_ATTACH, self.pid, 0, 0);
		#[cfg(target_os = "macos")]
		let ptrace_res = libc::ptrace(libc::PT_ATTACHEXC, self.pid, std::ptr::null_mut(), 0);

		if ptrace_res != 0 {
			return Err(PtraceLockError::PtraceAttach(std::io::Error::last_os_error()))
		}

		// wait until the signal is delivered
		let waitpid_res = libc::waitpid(self.pid, std::ptr::null_mut(), 0);
		if waitpid_res == -1 {
			return Err(PtraceLockError::WaitpidError(std::io::Error::last_os_error()))
		}
		debug_assert_eq!(waitpid_res, self.pid);

		self.ptrace_attached = true;

		Ok(())
	}

	unsafe fn ptrace_stop(&mut self) -> Result<(), PtraceLockError> {
		if libc::kill(self.pid, libc::SIGSTOP) != 0 {
			return Err(PtraceLockError::SigstopError(std::io::Error::last_os_error()))
		}

		// wait until the signal is delivered
		// TODO: read the manpage and check how to properly use this
		let waitpid_res = libc::waitpid(self.pid, std::ptr::null_mut(), 0);
		if waitpid_res == -1 {
			return Err(PtraceLockError::WaitpidError(std::io::Error::last_os_error()))
		}
		debug_assert_eq!(waitpid_res, self.pid);

		Ok(())
	}

	unsafe fn ptrace_cont(&mut self) -> Result<(), PtraceLockError> {
		#[cfg(target_os = "linux")]
		let ptrace_res = libc::ptrace(libc::PTRACE_CONT, self.pid, 0, 0);
		#[cfg(target_os = "macos")]
		let ptrace_res = libc::ptrace(libc::PT_CONTINUE, self.pid, std::ptr::null_mut(), 0);

		if ptrace_res != 0 {
			return Err(PtraceLockError::PtraceCont(std::io::Error::last_os_error()))
		}

		Ok(())
	}

	unsafe fn ptrace_detach(&mut self) -> Result<(), PtraceLockError> {
		debug_assert!(self.ptrace_attached);

		#[cfg(target_os = "linux")]
		let ptrace_res = libc::ptrace(libc::PTRACE_DETACH, self.pid, 0, 0);
		#[cfg(target_os = "macos")]
		let ptrace_res = libc::ptrace(libc::PT_DETACH, self.pid, std::ptr::null_mut(), 0);

		if ptrace_res != 0 {
			return Err(PtraceLockError::PtraceDetach(std::io::Error::last_os_error()))
		}
		self.ptrace_attached = false;

		Ok(())
	}
}
impl MemoryLock for PtraceLock {
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
}
impl Drop for PtraceLock {
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