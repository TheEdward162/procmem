use std::{sync::{Mutex, Arc, mpsc::{self, Sender, Receiver}}, thread::JoinHandle};
use std::ops::DerefMut;

use crate::{map::MemoryPageIndex, process::ProcessContext, scan::{ScanEntry, ScanFlow, base::{ScannerContextBase, ScanError}}};


#[derive(Debug)]
enum ScanThreadRequest {
	/// Issue a scan request
	Scan {
		page: MemoryPageIndex,
		unaligned: bool
	},
	/// Cancels all pending requests
	Cancel,
	/// Quit the thread
	Quit
}

pub struct MultithreadInstance {
	process: Arc<Mutex<ProcessContext>>,
	scanners: Vec<
		(Sender<ScanThreadRequest>, JoinHandle<()>)
	>,
	result_ch: (Sender<ScanEntry>, Receiver<ScanEntry>)
}
impl MultithreadInstance {
	pub fn new(process: ProcessContext) -> Self {
		let result_ch = mpsc::channel();
		
		MultithreadInstance {
			process: Arc::new(Mutex::new(process)),
			scanners: Vec::new(),
			result_ch
		}
	}

	/// Spawns a new scanner thread.
	pub fn spawn_scanner(&mut self) {
		let process = self.process.clone();
		let result_sender = self.result_ch.0.clone();

		let (request_sender, request_receiver) = mpsc::channel::<ScanThreadRequest>();

		let scanner_thread = std::thread::spawn(
			move || {
				let mut thread = ScannerThread::new(process, request_receiver, result_sender);
				thread.run();

			}
		);

		self.scanners.push(
			(request_sender, scanner_thread)
		);
	}
	
	/*
	/// Stops one of the scanner threads.
	// TODO: needs more design work
	pub fn stop_scanner(&mut self) {
		match self.scanners.pop() {
			None => (),
			Some((sender, handle)) => {
				sender.send(ScanThreadRequest::Quit).unwrap();
				handle.join().unwrap();
			}
		}
	}
	*/

	pub fn scan(
		&mut self,
		page: MemoryPageIndex,
		unaligned: bool,
		callback: impl FnMut(ScanEntry) -> ScanFlow
	) -> Result<(), ScanError> {
		unsafe {
			self.scanner.scan(
				&mut self.process,
				page,
				unaligned,
				callback
			)
		}
	}
}

#[derive(PartialEq)]
enum ScannerThreadFlow {
	Continue,
	Cancel,
	Quit
}
struct ScannerThread {
	process: Arc<Mutex<ProcessContext>>,
	receiver: Receiver<ScanThreadRequest>,
	sender: Sender<ScanEntry>,
	scanner: ScannerContextBase,

	request_queue: Vec<(MemoryPageIndex, bool)>
}
impl ScannerThread {
	pub fn new(
		process: Arc<Mutex<ProcessContext>>,
		receiver: Receiver<ScanThreadRequest>,
		sender: Sender<ScanEntry>
	) -> Self {
		let scanner = {
			let mut process_lock = process.lock().unwrap();

			ScannerContextBase::new(
				process_lock.deref_mut()
			).expect("could not create scanner context")
		};

		ScannerThread {
			process,
			receiver,
			sender,
			scanner,
			request_queue: Vec::new()
		}
	}

	// TODO: Error handling
	pub fn run(&mut self) {
		loop {
			// issue one blocking recv
			match self.receiver.recv().unwrap() {
				ScanThreadRequest::Quit => break,
				ScanThreadRequest::Cancel => {
					debug_assert_eq!(self.request_queue.len(), 0);
				}
				ScanThreadRequest::Scan { page, unaligned } => {
					self.request_queue.push((page, unaligned));
				}
			}

			// consume the queue
			match Self::consume_queue(&self.receiver, &mut self.request_queue) {
				ScannerThreadFlow::Quit => break,
				ScannerThreadFlow::Cancel => {
					debug_assert_eq!(self.request_queue.len(), 0);
				}
				ScannerThreadFlow::Continue => ()
			}

			if self.request_queue.len() > 0 {
				// TODO: Measure if this is a bottleneck
				let v = self.request_queue.remove(0);

				self.run_scan(v.0, v.1);
			}
		}
	}

	// TODO: Error handling
	fn run_scan(&mut self, page: MemoryPageIndex, unaligned: bool) {
		let entry = {
			let mut lock = self.process.lock().unwrap();
			lock.ptrace_attach().unwrap();

			lock.memory_map().page(page).unwrap().clone()
		};
		
		let receiver_ref = &self.receiver;
		let sender_ref = &self.sender;
		let request_queue_refmut = &mut self.request_queue;

		let result = unsafe {
			self.scanner.scan_raw(
				&entry,
				unaligned,
				|entry| {
					sender_ref.send(entry).unwrap();

					if Self::consume_queue(
						receiver_ref,
						request_queue_refmut
					) == ScannerThreadFlow::Continue {
						ScanFlow::Continue
					} else {
						ScanFlow::Break
					}
				}
			)
		};

		{
			let mut lock = self.process.lock().unwrap();
			lock.ptrace_detach().unwrap();
		}

		result.unwrap();
	}

	fn consume_queue(
		receiver: &Receiver<ScanThreadRequest>,
		request_queue: &mut Vec<(MemoryPageIndex, bool)>
	) -> ScannerThreadFlow {
		for request in receiver.try_iter() {
			match request {
				ScanThreadRequest::Quit => {
					return ScannerThreadFlow::Quit
				}
				ScanThreadRequest::Cancel => {
					request_queue.clear();
					return ScannerThreadFlow::Cancel;
				}
				ScanThreadRequest::Scan { page, unaligned } => {
					request_queue.push((page, unaligned));
				}
			}
		}

		ScannerThreadFlow::Continue
	}
}