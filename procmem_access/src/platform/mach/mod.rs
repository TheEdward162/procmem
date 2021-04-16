pub mod access;
pub mod map;
pub mod exception;

pub use access::MachAccess;
pub use map::MachMemoryMap;

#[derive(Debug, Default)]
pub struct TaskPort(mach::port::mach_port_name_t);
impl TaskPort {
	pub fn new(pid: libc::pid_t) -> Result<Self, std::io::Error> {
		let mut port = mach::port::MACH_PORT_NULL;

		let result = unsafe {
			mach::traps::task_for_pid(
				mach::traps::mach_task_self(),
				pid,
				&mut port
			)
		};
		if result != mach::kern_return::KERN_SUCCESS {
			if result == mach::kern_return::KERN_FAILURE {
				return Err(
					std::io::Error::new(
						std::io::ErrorKind::PermissionDenied,
						"could not access process"
					)
				);
			}

			return Err(
				std::io::Error::last_os_error()
			);
		}

		Ok(TaskPort(port))
	}

	/// ## Safety
	/// * `port` must be a valid port that needs to be deallocated on drop.
	unsafe fn from_raw(port: mach::port::mach_port_name_t) -> Self {
		TaskPort(port)
	}

	pub const fn get(&self) -> mach::port::mach_port_name_t {
		self.0
	}
}
impl Drop for TaskPort {
	fn drop(&mut self) {
		let result = unsafe {
			mach::mach_port::mach_port_deallocate(
				mach::traps::mach_task_self(),
				self.0
			)
		};

		debug_assert_eq!(result, 0);
	}
}