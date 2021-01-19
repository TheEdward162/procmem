use super::{ScanEntry, ScanFlow};

pub mod array;

/// Scan callback that is used to control and process output of the scanning.
#[allow(unused_variables)]
pub trait ScanCallback {
	/// Called for each scan entry.
	///
	/// Return value indicates to the scanner whether to continue scanning or end the scan.
	fn entry(&mut self, entry: ScanEntry) -> ScanFlow;

	/// Called for each first scanned data type on each page.
	///
	/// [entry](#ScanCallback::entry) is still called for each entry.
	fn page_start(&mut self, entry: ScanEntry) -> ScanFlow {
		ScanFlow::Continue
	}

	/// Called after each page.
	///
	/// `offset` is the last offset belonging to the page.
	fn page_end(&mut self, offset: crate::util::OffsetType) {}
}
impl<T: ScanCallback + ?Sized, D: std::ops::DerefMut<Target = T>> ScanCallback for D {
	fn entry(&mut self, entry: ScanEntry) -> ScanFlow {
		(**self).entry(entry)
	}

	fn page_start(&mut self, entry: ScanEntry) -> ScanFlow {
		(**self).page_start(entry)
	}

	fn page_end(&mut self, offset: crate::util::OffsetType) {
		(**self).page_end(offset)
	}
}

// TODO: fml, needs specialization I guess?
// impl<T: FnMut(ScanEntry) -> ScanFlow> ScanCallback for T {
// 	fn handle(&mut self, entry: ScanEntry) -> ScanFlow {
// 		self(entry)
// 	}
// }
pub struct ScanCallbackClosure<C: FnMut(ScanEntry) -> ScanFlow>(pub C);
impl<C: FnMut(ScanEntry) -> ScanFlow> ScanCallback for ScanCallbackClosure<C> {
	fn entry(&mut self, entry: ScanEntry) -> ScanFlow {
		(self.0)(entry)
	}
}
