use procmem::map::load_maps;

fn main() {
	// let mut global_context = GlobalContext::new

	let maps = load_maps("/proc/1196/maps").expect("could not load maps");

	for map in maps {
		println!("{}", map);
	}
}