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

// <https://opensource.apple.com/source/xnu/xnu-2422.1.72/libsyscall/wrappers/libproc/libproc.h.auto.html>
pub struct ProcessInfo {
	pub pid: libc::pid_t,
	pub name: String
}
impl ProcessInfo {
	pub fn list_all() -> std::io::Result<Vec<Self>> {
		let pids = {
			// get initial count
			let count = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
			if count < 0 {
				return Err(std::io::Error::last_os_error());
			}
			if count == 0 {
				return Ok(Vec::new());
			}
			
			// prepare destination buffer and read the actual pids
			let mut pids: Vec<libc::pid_t> = Vec::new();
			pids.resize(count as usize, 0);
			let count = unsafe {
				libc::proc_listallpids(
					pids.as_mut_ptr() as _,
					(pids.len() * std::mem::size_of::<libc::pid_t>()) as _
				)
			};
			if count < 0 {
				return Err(std::io::Error::last_os_error());
			}
			// in case the number of processes increased between the two reads, we return the smaller number
			let count = pids.len().min(count as usize);
			unsafe { pids.set_len(count); }
	
			pids
		};
	
		let mut processes = Vec::with_capacity(pids.len());
		for pid in pids {
			processes.push(Self::for_pid(pid)?);
		}
	
		Ok(processes)
	}

	pub fn for_pid(pid: libc::pid_t) -> std::io::Result<Self> {
		let name = Self::process_name(pid)?;
		Ok(Self { pid, name })
	}

	fn process_name(pid: libc::pid_t) -> std::io::Result<String> {	
		let mut buffer = [0u8; 32];
	
		let count = unsafe { libc::proc_name(pid, buffer.as_mut_ptr() as _, buffer.len() as _) };
		if count < 0 { return Err(std::io::Error::last_os_error()); }
	
		Ok(String::from_utf8_lossy(&buffer[..count as usize]).into_owned())
	}
}
