use thiserror::Error;

use crate::{map::MemoryPageIndex, process::{ProcessContext, ProcessContextError}, scan::{base::{ScanError, ScannerContextBase, ScannerContextBaseError}, callback::ScanCallback}};

#[derive(Debug, Error)]
pub enum SinglethreadInstanceError {
	#[error(transparent)]
	ProcessContextError(#[from] ProcessContextError),
	#[error(transparent)]
	ScannerContextBaseError(#[from] ScannerContextBaseError)
}

pub struct SinglethreadInstance {
	process: ProcessContext,
	scanner: ScannerContextBase
}
impl SinglethreadInstance {
	pub fn new(pid: libc::pid_t) -> Result<Self, SinglethreadInstanceError> {
		let mut process = ProcessContext::new(pid)?;
		let scanner = ScannerContextBase::new(&mut process)?;

		Ok(SinglethreadInstance { process, scanner })
	}

	pub fn process(&mut self) -> &mut ProcessContext {
		&mut self.process
	}

	pub fn scan(
		&mut self,
		page: MemoryPageIndex,
		unaligned: bool,
		callback: impl ScanCallback
	) -> Result<(), ScanError> {
		unsafe {
			self.scanner.scan(
				&mut self.process,
				page,
				unaligned,
				callback
			)
		}
	}
}
