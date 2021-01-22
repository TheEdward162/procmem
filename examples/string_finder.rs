use procmem::{memory::map::MemoryPage, platform::procfs::{map::ProcfsMemoryMap, access::ProcfsAccess}, scanner::{predicate::value::ValuePredicate, stream::StreamScanner}};
use procmem::memory::{access::MemoryAccess, map::{MemoryMap, MemoryPageType}};

fn main() {
	// simple cli parse
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

	// define what to scan for
	let predicate = ValuePredicate::new(needle, true);
	let mut scanner = StreamScanner::new(predicate);

	// create memory access and lock it so that the process gets frozen and we don't have races
	let mut memory_access = ProcfsAccess::open(
		pid
	).expect("could not open process memory");
	memory_access.lock().expect("could not lock memory access");

	// load up the memory map of the process
	let memory_map = ProcfsMemoryMap::load(
		pid
	).expect("could not read memory map");

	// filter pages to only include the original process executable (or whatever we want).
	// the run it through `MemoryPage::merge_sorted` so that consecutive pages get merged into one 
	let pages = MemoryPage::merge_sorted(
			memory_map.pages().iter().filter(
			|page| page.permissions.read() && match page.page_type {
				MemoryPageType::ProcessExecutable(_) => true,
				_ => false
			}
		).cloned()
	);

	// for each page, read it into the buffer then scan the chunk
	let mut page_buffer = Vec::new();
	for page in pages {
		page_buffer.resize(page.address_range[1].get() - page.address_range[0].get(), 0);
		eprintln!("Reading page {}", page);
		// Safe becasue the process is locked and thus cannot change until we unlock it
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

		// scan the chunk (one or more conscutive pages at once)
		scanner.scan_once(
			page.address_range[0],
			page_buffer.iter().copied(),
			|offset, len| {
				let relative_offset = offset.get() - page.address_range[0].get();
				
				println!(
					"[0x{}]: {}",
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

	// finally unlock the memory so that the process gets unfrozen
	// if we didn't call this `memory_access` would call it on drop anyway, but it's good practice to call it explicitly
	memory_access.unlock().expect("could not unlock memory access");
}