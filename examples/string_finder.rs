use procmem::{platform::procfs::{map::ProcfsMemoryMap, access::ProcfsAccess}, scanner::{predicate::value::ValuePredicate, sequential::SequentialScanner}};
use procmem::memory::{access::MemoryAccess, map::{MemoryMap, MemoryPageType}};

fn main() {
	let (pid, needle) = {
		let mut it = std::env::args().skip(1);

		let pid: libc::pid_t = it.next().and_then(
			|s| s.parse().ok()
		).unwrap_or_else(
			|| unsafe { libc::getpid() }
		);

		let needle = it.next().unwrap_or_else(|| "\x7FELF".to_string());

		(pid, needle)
	};
	eprintln!("pid: {}", pid);
	eprintln!("needle: {}", needle);

	let memory_map = ProcfsMemoryMap::load(
		pid
	).expect("could not read memory map");

	let mut memory_access = ProcfsAccess::open(
		pid
	).expect("could not open process memory");

	let predicate = ValuePredicate::new(needle, true);
	let mut scanner = SequentialScanner::new(predicate);

	let pages = memory_map.pages().iter().filter(
		|page| page.permissions.read() && match page.page_type {
			MemoryPageType::File(_) => true,
			_ => false
		}
	);

	let mut page_buffer = Vec::new();
	for page in pages {
		// memory_access.lock().expect("could not lock memory access");
		page_buffer.resize(page.address_range[1].get() - page.address_range[0].get(), 0);
		eprintln!("Reading page {}", page);
		unsafe {
			match memory_access.read(
				page.address_range[0],
				page_buffer.as_mut()
			) {
				Ok(()) => (),
				Err(err) => {
					eprintln!("could not read memory page {}", err);

					continue;
				}
			}
		}
		// memory_access.unlock().expect("could not unlock memory access");

		scanner.scan(
			page.address_range[0],
			page_buffer.iter().copied(),
			|offset, len| {
				let relative_offset = offset.get() - page.address_range[0].get();
				
				println!(
					"[{}]: {}",
					offset,
					std::str::from_utf8(
						&page_buffer[
							relative_offset .. relative_offset + len.get()
						]
					).unwrap()
				);

				true
			}
		);
	}
}