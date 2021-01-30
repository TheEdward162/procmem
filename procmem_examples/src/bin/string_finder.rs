use procmem_access::prelude::{
	MemoryAccess, MemoryLock,
	MemoryMap, MemoryPage, MemoryPageType
};
use procmem_access::platform::simple::{
	SimpleMemoryMap, SimpleMemoryLock, SimpleMemoryAccess
};
use procmem_scan::prelude::{
	ValuePredicate, StreamScanner
};

fn main() {
	// simple cli parse
	let (needle, pid) = {
		let mut it = std::env::args().skip(1);

		let needle = it.next().unwrap_or_else(|| "\x7FELF".to_string());

		let pid: i32 = it.next().and_then(
			|s| s.parse().ok()
		).unwrap_or_else(
			|| std::process::id() as i32
		);

		(needle, pid)
	};
	eprintln!("needle: {}", needle);
	eprintln!("pid: {}", pid);

	// define what to scan for
	let predicate = ValuePredicate::new(needle, true);
	let mut scanner = StreamScanner::new(predicate);

	// create and lock the memory lock so that the process gets frozen and we don't have races
	let mut memory_lock = SimpleMemoryLock::new(pid);
	memory_lock.lock().expect("could not lock process memory");

	// load up the memory map of the process
	let memory_map = SimpleMemoryMap::new(pid).expect("could not read memory map");

	// create memory access so we can read the memory
	let mut memory_access = SimpleMemoryAccess::new(pid).expect("could not open process memory");

	// filter pages to only include the original process executable (arbitrary filter).
	// and run it through `MemoryPage::merge_sorted` so that consecutive pages get merged into one 
	let pages = MemoryPage::merge_sorted(
		memory_map.pages().iter().filter(
			|page| page.permissions.read() && match page.page_type {
				MemoryPageType::ProcessExecutable(_) | MemoryPageType::Unknown => true,
				_ => false
			}
		).cloned()
	);

	// for each page, read it into the buffer then scan the chunk
	let mut chunk_buffer = Vec::new();
	for page in pages {
		chunk_buffer.resize((page.address_range[1].get() - page.address_range[0].get()) as usize, 0);
		eprintln!("Reading page {}", page);
		// Safe becasue the process is locked and thus cannot change until we unlock it
		// although even if we don't lock it, it should be ok to _read_ the memory
		// there just migh be a data race
		unsafe {
			match memory_access.read(
				page.address_range[0],
				chunk_buffer.as_mut()
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
			chunk_buffer.iter().copied()
		).for_each(
			|(offset, len)| {
				let relative_offset = (offset.get() - page.address_range[0].get()) as usize;
				
				println!(
					"[0x{}]: {}",
					offset,
					std::str::from_utf8(
						&chunk_buffer[
							relative_offset .. relative_offset + len.get()
						]
					).unwrap()
				);
			}
		);
	}

	// finally unlock the memory so that the process gets unfrozen
	// if we don't call this `memory_lock` would unlock on drop anyway, but it's good practice to call it explicitly
	memory_lock.unlock().expect("could not unlock memory access");
}