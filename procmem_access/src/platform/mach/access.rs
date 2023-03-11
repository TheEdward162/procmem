use thiserror::Error;

use mach::kern_return::KERN_SUCCESS;

use crate::{
	common::OffsetType,
	memory::access::{MemoryAccess, ReadError, WriteError},
};

#[derive(Debug, Error)]
pub enum MachAccessError {
	#[error("could not retrieve port handle")]
	PortError(std::io::Error),
}

pub struct MachAccess {
	#[allow(dead_code)]
	pid: libc::pid_t,
	port: super::TaskPort,
}
impl MachAccess {
	pub fn new(pid: libc::pid_t) -> Result<Self, MachAccessError> {
		let port = super::TaskPort::new(pid).map_err(MachAccessError::PortError)?;

		Ok(MachAccess { pid, port })
	}
}
impl MemoryAccess for MachAccess {
	unsafe fn read(&mut self, offset: OffsetType, buffer: &mut [u8]) -> Result<(), ReadError> {
		let mut read_len: u64 = 0;
		let res = mach::vm::mach_vm_read_overwrite(
			self.port.get(),
			offset.get(),
			buffer.len() as u64,
			buffer.as_mut_ptr() as u64,
			&mut read_len,
		);

		if res != KERN_SUCCESS {
			return Err(ReadError::Io(std::io::Error::last_os_error()));
		}

		// TODO: Can this happen? Why would this happen? Please don't let this happen.
		debug_assert_eq!(read_len, buffer.len() as u64);

		Ok(())
	}

	unsafe fn write(&mut self, offset: OffsetType, data: &[u8]) -> Result<(), WriteError> {
		let res = mach::vm::mach_vm_write(
			self.port.get(),
			offset.get(),
			data.as_ptr() as usize,
			data.len() as u32,
		);

		if res != KERN_SUCCESS {
			return Err(WriteError::Io(std::io::Error::last_os_error()));
		}

		Ok(())
	}
}
