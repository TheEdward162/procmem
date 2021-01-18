use crate::scan::ScanPrimitiveType;

use super::ScanCallback;
use super::super::{ScanFlow, ScanEntry};

#[derive(Debug)]
pub struct ArrayFinder<T: ScanPrimitiveType, A: AsRef<[T]>> {
	target: A,
	candidates: Vec<(usize, usize)>,
	found: Vec<usize>,
	_boo: std::marker::PhantomData<T>
}
impl<T: ScanPrimitiveType, A: AsRef<[T]>> ArrayFinder<T, A> {
	pub fn new(target: A) -> Self {
		ArrayFinder {
			target,
			candidates: Vec::new(),
			found: Vec::new(),
			_boo: std::marker::PhantomData
		}
	}

	fn on_entry(&mut self, offset: usize, element: T) {
		let target = self.target.as_ref();
		
		// go over candidate entries
		// if the entry fits, update it
		// if it fails, remove it
		let mut i = 0;
		while i < self.candidates.len() {
			let remove = {
				let mut candidate = &mut self.candidates[i];

				// if the offsets don't match, then we ignore this
				// right now this can only happen when using the same array finder for multiple pages
				if candidate.0 + candidate.1 == offset {
					// if the current element matches the expected candidate value
					if element == target[candidate.1] {
						candidate.1 += 1;
						if candidate.1 == target.len() {
							self.found.push(
								candidate.0
							);
							// remove the candidate because it has now been found
							true
						} else {
							false
						}
					} else {
						// remove the candidate because it doesn't match
						true
					}
				} else {
					false
				}
			};

			if remove {
				self.candidates.remove(i);
			} else {
				i += 1;
			}
		}
		
		// add new entry if the start matches
		if self.target.as_ref()[0] == element {
			self.candidates.push(
				(offset, 1)
			);
		}
	}

	/// Returns a slice of offsets at which arrays have been found.
	pub fn found(&self) -> &[usize] {
		&self.found
	}

	/*
	/// Returns a slice of tuples `(offset, pos)` at which possible candidates matching `target[..= pos]` have been found.
	pub fn candidates(&self) -> &[(usize, usize)] {
		&self.candidates
	}
	*/
}
impl<T: ScanPrimitiveType, A: AsRef<[T]>> ScanCallback for ArrayFinder<T, A> {
	fn handle(&mut self, entry: ScanEntry) -> ScanFlow {
		if let Some(element) = entry.data.try_cast::<T>() {
			self.on_entry(entry.offset, element);
		}

		ScanFlow::Continue
	}
}

#[cfg(test)]
mod test {
	use crate::scan::{ScanEntry, ScanFlow, callback::ScanCallback};
    use super::ArrayFinder;

	#[test]
	fn test_array_finder() {
		let value = b"Hello There";
		
		let mut finder = ArrayFinder::new(
			value
		);
		
		for (i, &byte) in value.into_iter().enumerate() {
			let res = finder.handle(ScanEntry::u8(i, byte));
			assert_eq!(res, ScanFlow::Continue);
		}

		assert_eq!(
			finder.found(),
			&[0]
		);
	}

	#[test]
	fn test_array_finder_multiple() {
		let data = [2u64, 1, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0, 1];
		
		let mut finder = ArrayFinder::new(
			[1u64, 0, 1, 0]
		);
		
		for (i, &value) in data.iter().enumerate() {
			let res = finder.handle(ScanEntry::u64(i, value));
			assert_eq!(res, ScanFlow::Continue);
		}
		assert_eq!(
			finder.found(),
			&[1, 7]
		);
	}

	#[test]
	fn test_array_find_multiple_pages() {
		let data = [2u64, 1, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0, 1];
		let second_data = [0u64, 1, 0];
		
		let mut finder = ArrayFinder::new(
			[1u64, 0, 1, 0]
		);
		
		for (i, &value) in data.iter().enumerate() {
			let res = finder.handle(ScanEntry::u64(i, value));
			assert_eq!(res, ScanFlow::Continue);
		}
		assert_eq!(
			finder.found(),
			&[1, 7]
		);

		for (i, &value) in second_data.iter().enumerate() {
			let res = finder.handle(ScanEntry::u64(i + 50, value));
			assert_eq!(res, ScanFlow::Continue);
		}
		assert_eq!(
			finder.found(),
			&[1, 7]
		);
	}
}