use procmem_access::prelude::{
	MemoryMap
};
use procmem_access::platform::simple::{
	SimpleMemoryMap
};

fn main() {
	// simple cli parse
	let pid = {
		let mut it = std::env::args().skip(1);

		let pid: i32 = it.next().and_then(
			|s| s.parse().ok()
		).unwrap_or_else(
			|| std::process::id() as i32
		);

		pid
	};
	eprintln!("pid: {}", pid);

	// load up the memory map of the process
	let memory_map = SimpleMemoryMap::new(pid).expect("could not read memory map");

	// print the map entries
	for page in memory_map.pages() {
		println!("{}", page);
	}
}