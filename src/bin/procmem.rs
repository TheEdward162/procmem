use procmem::{self, map::EntryType};

fn main() {
	let pid = unsafe { libc::getpid() };
	eprintln!("pid: {}", pid);
	let mut instance = procmem::instance::singlethread::SinglethreadInstance::new(pid).expect("Could not initialize instance");

	let pages = instance.process().memory_map().values().filter(
		|entry| match entry.entry_type {
			EntryType::Stack | EntryType::Heap | EntryType::Anon | EntryType::File(_) => true,
			_ => false
		}
	).inspect(|e| eprintln!("entry {}", e)).map(|entry| entry.address_range[0]).collect::<Vec<_>>();

	let mut array_finder = procmem::scan::callback::array::ArrayFinder::new(
		"Could not initialize instance"
	);

	for page in pages {
		eprintln!("Scanning page: {:x}", page);
		instance.scan(
			page,
			false,
			 &mut array_finder
		).unwrap();
	}

	for &found in array_finder.found() {
		let value = unsafe {
			let mut buffer = vec![0u8; 29];
			instance.process().read_memory(
				found,
				&mut buffer
			).unwrap();

			String::from_utf8(buffer).unwrap()
		};

		eprintln!("found: {:x}: \"{}\"", found, value);
	}
}
