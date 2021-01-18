use super::{ScanEntry, ScanFlow};

pub mod array;

/// Scan callback that is used to control and process output of the scanning.
pub trait ScanCallback {
	/// Handles one scan entry.
	///
	/// Return value indicates to the scanner whether to continue scanning or end the scan.
	fn handle(&mut self, entry: ScanEntry) -> ScanFlow;
}
impl<T: ScanCallback + ?Sized, D: std::ops::DerefMut<Target = T>> ScanCallback for D {
	fn handle(&mut self, entry: ScanEntry) -> ScanFlow {
		(**self).handle(entry)
	}
}

// TODO: fml, needs specialization I guess?
// impl<T: FnMut(ScanEntry) -> ScanFlow> ScanCallback for T {
// 	fn handle(&mut self, entry: ScanEntry) -> ScanFlow {
// 		self(entry)
// 	}
// }
pub struct ScanCallbackClosure<C: FnMut(ScanEntry) -> ScanFlow> {
	closure: C
}
impl<C: FnMut(ScanEntry) -> ScanFlow> ScanCallback for ScanCallbackClosure<C> {
	fn handle(&mut self, entry: ScanEntry) -> ScanFlow {
		(self.closure)(entry)
	}
}
impl<C: FnMut(ScanEntry) -> ScanFlow> From<C> for ScanCallbackClosure<C> {
	fn from(closure: C) -> Self {
		ScanCallbackClosure {
			closure
		}
	}
}