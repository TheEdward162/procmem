pub mod access;
pub mod map;

pub use access::ProcfsAccess;
pub use map::ProcfsMemoryMap;

pub struct ProcessInfo {
	pub pid: libc::pid_t,
	pub name: String
}
impl ProcessInfo {
	pub fn list_all() -> std::io::Result<Vec<Self>> {
		let mut processes = Vec::new();
	
		for entry in std::fs::read_dir("/proc/")? {
			let entry = entry?;

			if !entry.file_type()?.is_dir() {
				continue;
			}

			let pid = match entry.file_name().to_str().and_then(|e| e.parse::<libc::pid_t>().ok()) {
				None => continue,
				Some(p) => p
			};

			let info = match Self::for_pid(pid) {
				Err(_) => continue,
				Ok(i) => i
			};

			processes.push(info);
		}

		Ok(processes)
	}

	pub fn for_pid(pid: libc::pid_t) -> std::io::Result<Self> {
		let name = Self::process_name(pid)?;
		Ok(Self { pid, name })
	}

	fn process_name(pid: libc::pid_t) -> std::io::Result<String> {	
		std::fs::read_to_string(format!("/proc/{}/comm", pid)).map(
			|s| s.trim().into()
		)
	}
}
