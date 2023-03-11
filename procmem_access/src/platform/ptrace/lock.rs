use thiserror::Error;

use crate::memory::lock::{LockError, MemoryLock, UnlockError};

#[cfg(target_os = "macos")]
use crate::platform::mach::exception::{MachExceptionHandler, MachExceptionHandlerError};

#[derive(Debug, Error)]
pub enum PtraceLockError {
	#[error("ptrace attach failed")]
	PtraceAttach(std::io::Error),
	#[error("stopping failed")]
	StopError(std::io::Error),
	#[error("ptrace continue failed")]
	PtraceCont(std::io::Error),
	#[error("ptrace detach failed")]
	PtraceDetach(std::io::Error),

	#[cfg(target_os = "linux")]
	#[error("waitpid failed")]
	WaitpidError(std::io::Error),

	#[cfg(target_os = "macos")]
	#[error(transparent)]
	ExceptionHandlerError(#[from] MachExceptionHandlerError),
	#[cfg(target_os = "macos")]
	#[error("failed to initialize mach exception port")]
	ExceptionPortError(std::io::Error),
	#[cfg(target_os = "macos")]
	#[error("failed to receive mach exceptions")]
	ExceptionRecvError(std::io::Error),
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
	lock_counter: usize,

	#[cfg(target_os = "macos")]
	exception_handler: MachExceptionHandler,
}
#[cfg(target_os = "linux")]
impl PtraceLock {
	pub fn new(pid: libc::pid_t) -> Result<Self, PtraceLockError> {
		let mut me = PtraceLock {
			pid,
			lock_counter: 0,
		};

		unsafe { me.ptrace_attach()? };

		Ok(me)
	}

	unsafe fn wait_for_stop(&mut self) -> Result<(), PtraceLockError> {
		// wait until the stop signal is delivered
		// TODO: read the manpage and check how to properly use this
		let waitpid_res = libc::waitpid(self.pid, std::ptr::null_mut(), 0);
		if waitpid_res == -1 {
			return Err(PtraceLockError::WaitpidError(
				std::io::Error::last_os_error(),
			));
		}
		debug_assert_eq!(waitpid_res, self.pid);

		Ok(())
	}

	unsafe fn ptrace_attach(&mut self) -> Result<(), PtraceLockError> {
		let ptrace_res = libc::ptrace(libc::PTRACE_SEIZE, self.pid, 0, 0);
		if ptrace_res != 0 {
			return Err(PtraceLockError::PtraceAttach(
				std::io::Error::last_os_error(),
			));
		}

		Ok(())
	}

	unsafe fn ptrace_stop(&mut self) -> Result<(), PtraceLockError> {
		let ptrace_res = libc::ptrace(libc::PTRACE_INTERRUPT, self.pid, 0, 0);
		if ptrace_res != 0 {
			return Err(PtraceLockError::StopError(std::io::Error::last_os_error()));
		}
		self.wait_for_stop()?;

		Ok(())
	}

	unsafe fn ptrace_cont(&mut self) -> Result<(), PtraceLockError> {
		let ptrace_res = libc::ptrace(libc::PTRACE_CONT, self.pid, 0, 0);
		if ptrace_res != 0 {
			return Err(PtraceLockError::PtraceCont(std::io::Error::last_os_error()));
		}

		Ok(())
	}

	unsafe fn ptrace_detach(&mut self) -> Result<(), PtraceLockError> {
		let ptrace_res = libc::ptrace(libc::PTRACE_DETACH, self.pid, 0, 0);
		if ptrace_res != 0 {
			return Err(PtraceLockError::PtraceDetach(
				std::io::Error::last_os_error(),
			));
		}

		Ok(())
	}
}
#[cfg(target_os = "macos")]
impl PtraceLock {
	pub fn new(pid: libc::pid_t) -> Result<Self, PtraceLockError> {
		let mut me = PtraceLock {
			pid,
			lock_counter: 0,
			exception_handler: MachExceptionHandler::new(pid)?,
		};

		unsafe { me.ptrace_attach()? };

		Ok(me)
	}

	unsafe fn wait_for_stop(&mut self) -> Result<(), PtraceLockError> {
		while let Some(message) = self.exception_handler.try_receive() {
			dbg!(message);
		}

		Ok(())
	}

	unsafe fn ptrace_attach(&mut self) -> Result<(), PtraceLockError> {
		let ptrace_res = libc::ptrace(libc::PT_ATTACHEXC, self.pid, std::ptr::null_mut(), 0);
		if ptrace_res != 0 {
			return Err(PtraceLockError::PtraceAttach(
				std::io::Error::last_os_error(),
			));
		}
		self.wait_for_stop()?;
		self.ptrace_cont()?;

		Ok(())
	}

	unsafe fn ptrace_stop(&mut self) -> Result<(), PtraceLockError> {
		if libc::kill(self.pid, libc::SIGSTOP) != 0 {
			return Err(PtraceLockError::StopError(std::io::Error::last_os_error()));
		}
		self.wait_for_stop()?;

		Ok(())
	}

	unsafe fn ptrace_cont(&mut self) -> Result<(), PtraceLockError> {
		let ptrace_res = libc::ptrace(libc::PT_CONTINUE, self.pid, 1 as _, 0);
		if ptrace_res != 0 {
			return Err(PtraceLockError::PtraceCont(std::io::Error::last_os_error()));
		}

		Ok(())
	}

	unsafe fn ptrace_detach(&mut self) -> Result<(), PtraceLockError> {
		let ptrace_res = libc::ptrace(libc::PT_DETACH, self.pid, std::ptr::null_mut(), 0);
		if ptrace_res != 0 {
			return Err(PtraceLockError::PtraceDetach(
				std::io::Error::last_os_error(),
			));
		}

		Ok(())
	}
}
impl MemoryLock for PtraceLock {
	fn lock(&mut self) -> Result<bool, LockError> {
		if self.lock_counter == 0 {
			unsafe {
				self.ptrace_stop()?;
			}
			self.lock_counter = 1;

			Ok(true)
		} else if self.lock_counter == usize::MAX {
			Err(LockError::AlreadyLocked)
		} else {
			self.lock_counter += 1;

			Ok(false)
		}
	}

	fn lock_exlusive(&mut self) -> Result<(), LockError> {
		if self.lock_counter == 0 {
			self.lock()?;
			self.lock_counter = usize::MAX;

			Ok(())
		} else {
			Err(LockError::AlreadyLocked)
		}
	}

	fn unlock(&mut self) -> Result<bool, UnlockError> {
		if self.lock_counter == 0 {
			return Err(UnlockError::NotLocked);
		}

		if self.lock_counter == 1 || self.lock_counter == usize::MAX {
			unsafe {
				self.ptrace_cont()?;
			}
			self.lock_counter = 0;

			Ok(true)
		} else {
			self.lock_counter -= 1;

			Ok(false)
		}
	}
}
impl Drop for PtraceLock {
	fn drop(&mut self) {
		let _ = self.lock();

		unsafe { self.ptrace_detach().unwrap() }
	}
}
