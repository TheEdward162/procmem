use anyhow::Context;
use rustyline::{
	completion::Pair as CompletionPair, config::Config, error::ReadlineError, history::MemHistory,
	Editor,
};

struct ReplHelper {}
impl ReplHelper {
	pub fn new() -> Self {
		Self {}
	}

	fn try_complete(line: &str) -> Vec<CompletionPair> {
		let mut results = Vec::new();

		macro_rules! complete_to {
			(
				$( $command: literal),+ $(,)?
			) => {
				$(
					if $command.starts_with(line) {
						results.push(CompletionPair {
							display: $command.into(),
							replacement: $command.into()
						});
					}
				)+
			};
		}
		complete_to! {
			"reset",
			"detach",
			"attach ",
			"scan i16 ",
			"scan i32 ",
			"scan i64 ",
			"scan f32 ",
			"scan f64 ",
			"scan all ",
			"write i16 ",
			"write i32 ",
			"write i64 ",
			"write f32 ",
			"write f64 ",
			"stop",
			"continue",
			"info",
			"info pages",
			"exit"
		}

		results
	}
}
impl rustyline::validate::Validator for ReplHelper {}
impl rustyline::highlight::Highlighter for ReplHelper {}
impl rustyline::hint::Hinter for ReplHelper {
	type Hint = String;

	fn hint(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> Option<Self::Hint> {
		if line.len() == 0 {
			return None;
		}

		let completions = Self::try_complete(line);

		match completions.get(0) {
			None => None,
			Some(completion) => Some(completion.replacement[pos..].to_string()),
		}
	}
}
impl rustyline::completion::Completer for ReplHelper {
	type Candidate = CompletionPair;

	fn complete(
		&self,
		line: &str,
		_pos: usize,
		_ctx: &rustyline::Context<'_>,
	) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
		if line.starts_with("attach ") {
			const MAX_SHOWN: usize = 16;

			let pid_prefix = line.split_whitespace().nth(1);
			let pids = app::ProcessInfo::list_all()
				.unwrap()
				.into_iter()
				.filter_map(|p| {
					let pid_str = format!("{}", p.pid);
					let display = if p.name.len() > MAX_SHOWN {
						format!("{} ({}~)", pid_str, &p.name[..(MAX_SHOWN - 1)])
					} else {
						format!("{} ({})", pid_str, p.name)
					};

					match pid_prefix.map(|prefix| pid_str.starts_with(prefix)) {
						Some(false) => None,
						Some(true) | None => Some(CompletionPair {
							display,
							replacement: pid_str,
						}),
					}
				})
				.collect();

			return Ok((7, pids));
		}

		Ok((0, Self::try_complete(line)))
	}
}
impl rustyline::Helper for ReplHelper {}

fn main() -> anyhow::Result<()> {
	const PROMPT: &str = "> ";

	let mut rl = Editor::<ReplHelper, MemHistory>::with_history(
		Config::builder()
			.completion_type(rustyline::CompletionType::List)
			.auto_add_history(true)
			.bell_style(rustyline::config::BellStyle::None)
			.tab_stop(4)
			.build(),
		MemHistory::default(),
	)?;
	rl.set_helper(Some(ReplHelper::new()));

	let mut app: Option<App> = None;
	loop {
		macro_rules! on_attached {
			($app: ident => $($code: tt)+) => {
				match app {
					None => println!("Not attached, use `attach PID` first"),
					Some(ref mut $app) => {
						$($code)+
					}
				}
			};
		}

		match rl.readline(PROMPT) {
			Err(ReadlineError::Eof) => break,
			Err(ReadlineError::Interrupted) => break,
			Ok(line) if line == "exit" => break,
			Err(err) => anyhow::bail!("Failed to read line: {}", err),
			// commands
			Ok(line) if line.starts_with("attach ") => match app {
				Some(_) => println!("Already attached, use `detach` first"),
				None => match line.split_whitespace().nth(1).unwrap_or("").parse() {
					Err(_) => println!("Invalid PID"),
					Ok(pid) => {
						app = Some(App::attach(pid)?);
					}
				},
			},
			Ok(line) if line == "detach" => match app.take() {
				None => println!("Not attached, cannot detach"),
				Some(_) => (),
			},
			Ok(line) if line == "stop" => on_attached! { app => app.lock(); },
			Ok(line) if line == "continue" => on_attached! { app => app.unlock(); },
			Ok(line) if line == "reset" => on_attached! { app => app.reset(); },
			Ok(line) if line == "info" => on_attached! { app =>
				println!("PID: {}", app.process_info().pid);
				println!("Name: {}", app.process_info().name);
				println!("Pages:");
				for (_, page) in app.pages().filter(|(selected, _)| *selected) {
					println!("\t{}", page);
				}
				println!("Locked: {}", app.is_locked());
			},
			Ok(line) if line == "info pages" => on_attached! { app =>
				println!("Pages:");
				for (selected, page) in app.pages() {
					println!("\t[{}] {}", selected.then_some("x").unwrap_or(" "), page);
				}
			},
			// scans
			Ok(line) if line.starts_with("scan ") => on_attached! { app =>
				let mut arguments = line.split_whitespace().skip(1);

				let value_type = arguments.next().context("scan type is required")?;
				let value_str = arguments.next().context("scan value is required")?;

				let mut aligned = true;
				let mut swapped_bytes = false;
				for argument in arguments {
					match argument {
						"unalign" => { aligned = false; }
						"swap" => { swapped_bytes = true; }
						flag => anyhow::bail!("Invalid scan flag \"{}\"", flag)
					}
				}

				macro_rules! do_scan {
					($scan_type: ty) => {
						{
							println!("Scanning as {} (align: {}, swap: {})...", stringify!($scan_type), aligned, swapped_bytes);
							match value_str.parse::<$scan_type>() {
								Err(err) => println!("Skipping scan: {}", err),
								Ok(value) => {
									let value = if swapped_bytes {
										#[cfg(target_endian = "little")]
										{ value.to_be_bytes() }
										#[cfg(target_endian = "big")]
										{ value.to_le_bytes() }
									} else {
										value.to_ne_bytes()
									};

									match app.scan_exact(value, aligned)? {
										ScanResult::Zero => { println!("No matches"); },
										ScanResult::One(offset) => println!("One match: 0x{}", offset),
										ScanResult::Few(offsets) => println!("{} matches: {:X?}", offsets.len(), offsets),
										ScanResult::Many(n) => println!("{} matches", n)
									}
								}
							}
						}
					};
				}

				match value_type {
					"all" => {
						do_scan!(i16);
						app.reset();
						do_scan!(i32);
						app.reset();
						do_scan!(i64);
						app.reset();
						do_scan!(f32);
						app.reset();
						do_scan!(f64);
						app.reset();
					}
					"i16" => do_scan!(i16),
					"i32" => do_scan!(i32),
					"i64" => do_scan!(i64),
					"f32" => do_scan!(f32),
					"f64" => do_scan!(f64),
					value_type => anyhow::bail!("Unknown value type \"{}\"", value_type)
				}
			},
			Ok(line) if line.starts_with("write ") => on_attached! { app =>
				let mut arguments = line.split_whitespace().skip(1);

				let value_type = arguments.next().context("write type is required")?;
				let offset = arguments.next().and_then(|v| u64::from_str_radix(v, 16).ok()).context("write offset is required")?;
				let value_str = arguments.next().context("write value is required")?;

				macro_rules! do_write {
					($write_type: ty) => {
						{
							match value_str.parse::<$write_type>() {
								Err(err) => println!("Skipping write: {}", err),
								Ok(value) => unsafe { app.write(offset, value)? }
							}
						}
					};
				}

				match value_type {
					"i16" => do_write!(i16),
					"i32" => do_write!(i32),
					"i64" => do_write!(i64),
					"f32" => do_write!(f32),
					"f64" => do_write!(f64),
					value_type => anyhow::bail!("Unknown value type \"{}\"", value_type)
				}
			},
			// rest
			Ok(line) => println!("Unknown command \"{}\"", line),
		}
	}

	Ok(())
}

mod app {
	use std::collections::BTreeSet;

	use anyhow::Context;

	pub use procmem_access::platform::simple::ProcessInfo;
	use procmem_access::{
		platform::simple::{SimpleMemoryAccess, SimpleMemoryLock, SimpleMemoryMap},
		prelude::{MemoryAccess, MemoryLock, MemoryMap, MemoryPage, OffsetType},
	};
	use procmem_scan::prelude::{ByteComparable, StreamScanner, ValuePredicate};

	pub enum ScanResult {
		Many(usize),
		Few(Vec<OffsetType>),
		One(OffsetType),
		Zero,
	}

	pub struct App {
		pid: i32,
		lock: SimpleMemoryLock,
		#[allow(dead_code)]
		map: SimpleMemoryMap,
		access: SimpleMemoryAccess,
		pages: Vec<MemoryPage>,
		current_matches: BTreeSet<OffsetType>,
		user_locked: bool,
	}
	impl App {
		fn filter_page_predicate(page: &MemoryPage) -> bool {
			page.permissions.read()
				&& page.permissions.write()
				&& !page.permissions.shared()
				&& page.offset == 0

			// && matches!(page.page_type, MemoryPageType::Stack | MemoryPageType::Heap)
		}

		pub fn attach(pid: i32) -> anyhow::Result<Self> {
			let mut lock = SimpleMemoryLock::new(pid)?;
			lock.lock()?;

			let map = SimpleMemoryMap::new(pid)?;
			let access = SimpleMemoryAccess::new(pid)?;

			let pages: Vec<MemoryPage> = MemoryPage::merge_sorted(
				map.pages()
					.into_iter()
					.filter(|page| Self::filter_page_predicate(page))
					.cloned(),
			)
			.collect();

			lock.unlock()?;

			Ok(Self {
				pid,
				lock,
				map,
				access,
				pages,
				current_matches: Default::default(),
				user_locked: false,
			})
		}

		pub fn process_info(&self) -> ProcessInfo {
			ProcessInfo::for_pid(self.pid).unwrap()
		}

		pub fn pages(&self) -> impl Iterator<Item = (bool, &'_ MemoryPage)> {
			self.map
				.pages()
				.into_iter()
				.map(|p| (Self::filter_page_predicate(p), p))
		}

		pub fn is_locked(&self) -> bool {
			self.user_locked
		}

		pub fn lock(&mut self) {
			if self.user_locked {
				return;
			}
			self.user_locked = true;

			self.lock.lock().unwrap();
		}

		pub fn unlock(&mut self) {
			if !self.user_locked {
				return;
			}
			self.user_locked = false;

			self.lock.unlock().unwrap();
		}

		pub fn reset(&mut self) {
			self.current_matches.clear()
		}

		pub fn scan_exact<T: ByteComparable>(
			&mut self,
			value: T,
			aligned: bool,
		) -> anyhow::Result<ScanResult> {
			self.lock.lock()?;

			let predicate = ValuePredicate::new(value, aligned);
			let mut scanner = StreamScanner::new(predicate);

			let mut new_matches = BTreeSet::default();
			let mut chunk_buffer = Vec::new();
			for page in self.pages.iter() {
				chunk_buffer.resize(page.size() as usize, 0);

				unsafe {
					self.access
						.read(page.start(), chunk_buffer.as_mut())
						.context("Could not read memory page")?;
				}

				for (offset, _) in scanner.scan_once(page.start(), chunk_buffer.iter().copied()) {
					if self.current_matches.len() == 0 || self.current_matches.contains(&offset) {
						new_matches.insert(offset);
					}
				}
			}
			self.current_matches = new_matches;

			let result = match self.current_matches.len() {
				0 => ScanResult::Zero,
				1 => ScanResult::One(self.current_matches.iter().next().unwrap().clone()),
				2..=5 => ScanResult::Few(self.current_matches.iter().cloned().collect()),
				n => ScanResult::Many(n),
			};

			self.lock.unlock()?;

			Ok(result)
		}

		pub unsafe fn write<T: ByteComparable>(
			&mut self,
			offset: u64,
			value: T,
		) -> anyhow::Result<()> {
			self.lock.lock()?;

			let offset = OffsetType::new_unwrap(offset);

			unsafe {
				self.access
					.write(offset, value.as_bytes())
					.context("Could not write memory")?
			};

			self.lock.unlock()?;
			Ok(())
		}
	}
}
use app::{App, ScanResult};
