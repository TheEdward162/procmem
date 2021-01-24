use std::num::NonZeroUsize;

use procmem_access::prelude::OffsetType;

use crate::candidate::ScannerCandidate;
use crate::predicate::{ScannerPredicate, UpdateCandidateResult, PartialScannerPredicate};

/// Scans a stream of bytes for values matching the predicate.
pub struct StreamScanner<P: ScannerPredicate> {
	predicate: P,
	candidates: Vec<ScannerCandidate>
}
impl<P: ScannerPredicate> StreamScanner<P> {
	pub fn new(predicate: P) -> Self {
		StreamScanner {
			predicate,
			candidates: Vec::new()
		}
	}

	/// Resets this scanner.
	///
	/// For normal scans, this has no effect.
	/// For partial scans, this clears existing progress from previous partial scans.
	pub fn reset(&mut self) {
		self.candidates.clear()
	}

	/// Runs the scanner on a stream.
	///
	/// Does not detect across multiple calls.
	pub fn scan_once(
		&mut self,
		offset: OffsetType,
		stream: impl Iterator<Item = u8>,
		mut callback: impl FnMut(OffsetType, NonZeroUsize) -> bool
	) {
		self.reset();

		let mut offset = offset;
		for byte in stream {
			let cont = self.on_byte(
				offset,
				byte,
				&mut callback
			);

			if !cont {
				break;
			}

			offset = offset.saturating_add(1);
		}
	}

	fn on_byte(
		&mut self,
		offset: OffsetType,
		byte: u8,
		mut callback: impl FnMut(OffsetType, NonZeroUsize) -> bool
	) -> bool {
		let mut i = 0;
		while i < self.candidates.len() {
			// make sure to skip candidates that are in a different address range
			if self.candidates[i].end_offset().get() != offset.get() {
				i += 1;
				continue;
			}

			match self.predicate.update_candidate(offset, byte, &self.candidates[i]) {
				UpdateCandidateResult::Advance => {
					self.candidates[i].advance();
					i += 1;
				}
				UpdateCandidateResult::Skip => {
					i += 1;
				}
				UpdateCandidateResult::Remove => {
					self.candidates.remove(i);
				}
				UpdateCandidateResult::Resolve if self.candidates[i].is_partial() => {
					// Treat resolve as skip if it is partial
					i += 1;
				}
				UpdateCandidateResult::Resolve => {
					let mut candidate = self.candidates.remove(i);
					candidate.advance();

					let cont = callback(
						candidate.offset(),
						candidate.length()
					);

					if !cont {
						return false;
					}
				}
			}
		}
		
		match self.predicate.try_start_candidate(offset, byte) {
			None => (),
			Some(candidate) => self.candidates.push(candidate)
		};

		true
	}
}
impl<P: PartialScannerPredicate> StreamScanner<P> {
	/// Runs the scanner on the sequence, preserving partial candidates.
	///
	/// Running this scan multiple times on chunks of a contiguous sequence will
	/// find matches the same way as if it was run on the whole sequence using [`scan`](StreamScanner::scan)
	pub fn scan_partial(
		&mut self,
		sequence_offset: OffsetType,
		mut sequence: impl Iterator<Item = u8>,
		mut callback: impl FnMut(OffsetType, NonZeroUsize) -> bool
	) {
		let mut offset = sequence_offset;
		
		// unroll the first iteration to run `on_start` here.
		if !sequence.next().map(|first_byte| {
			self.on_start(offset, first_byte);

			let cont = self.on_byte(
				offset,
				first_byte,
				&mut callback
			);
			if !cont {
				return false;
			}
			offset = offset.saturating_add(1);

			true
		}).unwrap_or(false) {
			return;
		}

		for byte in sequence {
			let cont = self.on_byte(
				offset,
				byte,
				&mut callback
			);

			if !cont {
				break;
			}
			offset = offset.saturating_add(1);
		}

		self.on_end();
	}
	
	/// Merges the partial candidates from other scanner into self.
	///
	/// This has the same effect as replaying the same scans that were run on `other` on self.
	///
	/// `other` is not modified. To clear the old matches from it, run [`reset`](StreamScanner::reset).
	pub fn merge_mut(
		&mut self,
		other: &Self,
		callback: impl FnMut(OffsetType, NonZeroUsize) -> bool
	) {
		todo!()
	}

	fn on_start(&mut self, offset: OffsetType, byte: u8) {
		self.candidates.extend(
			self.predicate.try_start_partial_candidates(offset, byte)
		);
	}

	fn on_end(&mut self) {
		dbg!(&self.candidates);
		// TODO: Probably nothing to do here
	}
}

// fn on_page_end(&mut self, offset: OffsetType) {
// 	// TODO: Oh unstable, why must you hurt me so
// 	// self.candidates.drain_filter(filter)

// 	// remove all candidates that aren't partial and don't end at the page boundary
// 	let mut i = 0;
// 	while i < self.candidates.len() {
// 		let remove = {
// 			let mut candidate = &mut self.candidates[i];

// 			// don't remove partial candidates
// 			if candidate.partial_offset.is_some() {
// 				false
// 			} else {
// 				if candidate.end_offset() == offset {
// 					candidate.partial_offset = Some(candidate.offset);
// 					// don't remove, ends at page boundary
// 					false
// 				} else {
// 					// remove all other candidates
// 					true
// 				}
// 			}
// 		};

// 		if remove {
// 			self.candidates.remove(i);
// 		} else {
// 			i += 1;
// 		}
// 	}
// }

/*
	pub fn merge<B: AsRef<[T]>>(&mut self, other: ArrayFinder<T, B>) {
		debug_assert_eq!(self.target.as_ref(), other.target.as_ref());
		debug_assert!(self.candidates.as_slice().windows(2).all(|w| {
			w[0].partial_cmp(&w[1])
				.map(|o| o != std::cmp::Ordering::Greater)
				.unwrap_or(false)
		}));
		debug_assert!(other.candidates.as_slice().windows(2).all(|w| {
			w[0].partial_cmp(&w[1])
				.map(|o| o != std::cmp::Ordering::Greater)
				.unwrap_or(false)
		}));
		// TODO: Use .is_sorted() when it becomes stable
		// debug_assert!(other.candidates.is_sorted());

		// merge candidates
		{
			let mut old_candidates =
				Vec::with_capacity(self.candidates.len() + other.candidates.len());
			std::mem::swap(&mut self.candidates, &mut old_candidates);

			let merge = MergeIter::new(old_candidates.into_iter(), other.candidates.into_iter());

			let mut maybe_current: Option<Candidate> = None;
			for cand in merge {
				if let Some(ref mut current) = maybe_current {
					// deduplicate
					if *current == cand {
						continue
					}

					// try merging
					match current.try_merge(cand) {
						Ok(()) => {
							// promote to found
							if current.partial_len().unwrap() == self.target.as_ref().len() {
								debug_assert_eq!(current.offset, current.partial_offset.unwrap());
								debug_assert_eq!(current.position, self.target.as_ref().len() - 1);
								self.found.push(current.offset);

								maybe_current = None;
							}
						}
						Err(mut cand) => {
							std::mem::swap(current, &mut cand);
							// if merge fails, then the current candidate cannot be merged at all
							// otherwise it would have been followed by a mergeable candidate
							// it is also not a duplicate since we check that above
							self.candidates.push(cand);
						}
					}
				} else {
					maybe_current = Some(cand);
				}
			}
			// add the remaining candidate
			if let Some(current) = maybe_current {
				self.candidates.push(current);
			}
		}

		{
			self.found.sort_unstable();
			let mut old_found = Vec::with_capacity(self.found.len() + other.found.len());
			std::mem::swap(&mut self.found, &mut old_found);

			let merge = MergeIter::new(old_found.into_iter(), other.found.into_iter());

			self.found.extend(merge);
			self.found.dedup();
		}
	}
*/

#[cfg(test)]
mod test {
	use std::num::NonZeroUsize;
	use std::convert::TryInto;

    use crate::predicate::value::ValuePredicate;
	use super::StreamScanner;
	use crate::common::AsRawBytes;

	#[test]
	fn test_array_finder() {
		let data = b"Hello There";

		let predicate = ValuePredicate::new(data, true);
		let mut scanner = StreamScanner::new(predicate);
		let mut found = Vec::new();

		scanner.scan_once(
			1.into(),
			data.iter().copied(),
			|offset, len| {
				found.push((offset, len));

				true
			}
		);

		assert_eq!(
			found,
			&[
				(1.into(), NonZeroUsize::new(data.len()).unwrap())
			]
		);
	}

	#[test]
	fn test_array_finder_multiple() {
		let data = [2u64, 1, 0, 1, 0, 1, 0, 0, 1, 0, 1, 0, 2];

		let predicate = ValuePredicate::new([1u64, 0, 1, 0], true);
		let mut scanner = StreamScanner::new(predicate);
		let mut found = Vec::new();

		scanner.scan_once(
			8.into(),
			data.as_raw_bytes().iter().copied(),
			|offset, len| {
				found.push((offset, len));

				true
			}
		);

		assert_eq!(
			found,
			&[
				(16.into(), 32.try_into().unwrap()),
				(32.into(), 32.try_into().unwrap()),
				(72.into(), 32.try_into().unwrap())
			]
		);
	}

	#[test]
	fn test_scan_scan_partial_equal() {
		let data = [3u8, 4, 3, 4, 5, 6];
		let predicate = ValuePredicate::new([3u8, 4], true);
		
		let mut scanner = StreamScanner::new(predicate);

		let mut found_scan = Vec::new();
		let mut found_scan_partial = Vec::new();

		scanner.scan_once(
			1.into(),
			data.iter().copied(),
			|offset, len| {
				found_scan.push((offset, len));

				true
			}
		);

		scanner.scan_partial(
			4.into(),
			data[3 ..].iter().copied(),
			|offset, len| {
				found_scan_partial.push((offset, len));

				true
			}
		);
		scanner.scan_partial(
			1.into(),
			data[.. 3].iter().copied(),
			|offset, len| {
				found_scan_partial.push((offset, len));

				true
			}
		);
		
		assert_eq!(
			found_scan,
			found_scan_partial
		);
	}

	/*
	#[test]
	fn test_array_finder_multiple_pages() {
		let data = [2u64, 1, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0, 1];
		let second_data = [0u64, 1, 0];

		let mut finder = ArrayFinder::new([1u64, 0, 1, 0]);

		for (i, &value) in data.iter().enumerate() {
			let res = finder.entry(ScanEntry::u64((i + 10).into(), value));
			assert_eq!(res, ScanFlow::Continue);
		}
		assert_eq!(finder.found(), &[11.into(), 17.into()]);

		for (i, &value) in second_data.iter().enumerate() {
			let res = finder.entry(ScanEntry::u64((i + 50).into(), value));
			assert_eq!(res, ScanFlow::Continue);
		}
		assert_eq!(finder.found(), &[11.into(), 17.into()]);
	}

	#[test]
	fn test_array_finder_merge() {
		const BASE_OFFSET: usize = 10;

		let target = [3.0f32, 4.0, 5.0, 6.0, 7.0, 8.0];
		let first_page = [3.0f32, 4.0, 5.0, 6.0, 7.0, 8.0, 1.0, 2.0, 3.0, 4.0];
		let second_page = [5.0f32, 6.0];
		let third_page = [7.0f32, 8.0, 9.0];

		fn simulate_scan_page<T: ScanPrimitiveType, A: AsRef<[T]>>(
			finder: &mut ArrayFinder<T, A>,
			base_offset: usize,
			page: &[f32]
		) {
			let res = finder.page_start(ScanEntry::f32(base_offset.into(), page[0]));
			assert_eq!(res, ScanFlow::Continue);

			for (i, &value) in page.iter().enumerate() {
				let res = finder.entry(ScanEntry::f32((base_offset + i).into(), value));
				assert_eq!(res, ScanFlow::Continue);
			}

			finder.page_end((base_offset + page.len() - 1).into());
		}

		let mut first_finder = ArrayFinder::new(target);
		simulate_scan_page(&mut first_finder, BASE_OFFSET, &first_page);
		assert_eq!(
			first_finder.candidates,
			&[Candidate {
				offset: 18.into(),
				position: 1,
				partial_offset: Some(18.into())
			}]
		);
		assert_eq!(first_finder.found(), &[10.into()]);

		let mut second_finder = ArrayFinder::new(target);
		simulate_scan_page(
			&mut second_finder,
			BASE_OFFSET + first_page.len(),
			&second_page
		);
		assert_eq!(
			second_finder.candidates,
			&[Candidate {
				offset: 18.into(),
				position: 3,
				partial_offset: Some(20.into())
			}]
		);
		assert_eq!(second_finder.found(), &[]);

		let mut third_finder = ArrayFinder::new(target);
		simulate_scan_page(
			&mut third_finder,
			BASE_OFFSET + first_page.len() + second_page.len(),
			&third_page
		);
		assert_eq!(
			third_finder.candidates,
			&[Candidate {
				offset: 18.into(),
				position: 5,
				partial_offset: Some(22.into())
			}]
		);
		assert_eq!(third_finder.found(), &[]);

		first_finder.merge(second_finder);
		assert_eq!(
			first_finder.candidates,
			&[Candidate {
				offset: 18.into(),
				position: 3,
				partial_offset: Some(18.into())
			}]
		);
		assert_eq!(first_finder.found(), &[BASE_OFFSET.into()]);

		first_finder.merge(third_finder);
		assert_eq!(
			first_finder.found(),
			&[BASE_OFFSET.into(), (BASE_OFFSET + 8).into()]
		);
	}
	*/
}