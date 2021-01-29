pub mod access;
pub mod map;

pub use access::MachAccess;
pub use map::MachMemoryMap;

fn get_pid_port(pid: libc::pid_t) -> Result<mach::port::mach_port_name_t, std::io::Error> {
	let mut port = mach::port::MACH_PORT_NULL;

	unsafe {
		let result = mach::traps::task_for_pid(
			mach::traps::mach_task_self(),
			pid,
			&mut port
		);
		if result != mach::kern_return::KERN_SUCCESS {
			return Err(
				std::io::Error::last_os_error()
			);
		}
	}

	Ok(port)
}