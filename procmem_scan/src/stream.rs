use std::num::NonZeroUsize;

use procmem_access::{prelude::OffsetType, util::AccFilter};

use crate::{candidate::ScannerCandidate, predicate::{PartialScannerPredicate, ScannerPredicate, UpdateCandidateResult}};

pub type ScanResult = (OffsetType, NonZeroUsize);

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
	/// Does not detect across multiple calls and calls [`reset`](StreamScanner::reset) before and after scanning.
	pub fn scan_once<I: Iterator<Item = u8>>(
		&mut self,
		offset: OffsetType,
		stream: I
	) -> StreamScannerIter<'_, P, I> {
		self.reset();

		StreamScannerIter::new(
			self,
			offset,
			stream
		)
	}

	fn on_byte(
		&mut self,
		offset: OffsetType,
		byte: u8,
		found: &mut Vec<(OffsetType, NonZeroUsize)>
	) {
		let mut i = 0;
		while i < self.candidates.len() {
			let current = &self.candidates[i];

			// make sure to skip candidates that are in a different address range or that are resolved
			if current.is_resolved() || current.end_offset().get() != offset.get() {
				i += 1;
				continue
			}

			match self
				.predicate
				.update_candidate(offset, byte, current)
			{
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
				UpdateCandidateResult::Resolve if current.is_partial() => {
					// Treat resolve as skip if it is partial
					i += 1;
				}
				UpdateCandidateResult::Resolve => {
					let mut candidate = self.candidates.remove(i);
					candidate.resolve();

					found.push(
						(candidate.offset(), candidate.length())
					);
				}
			}
		}

		match self.predicate.try_start_candidate(offset, byte) {
			None => (),
			Some(candidate) if candidate.is_resolved() => {
				found.push(
					(candidate.offset(), candidate.length())
				);
			}
			Some(candidate) => self.candidates.push(candidate)
		};
	}
}
impl<P: PartialScannerPredicate> StreamScanner<P> {
	/// Runs the scanner on the sequence, preserving partial candidates.
	///
	/// Running this scan multiple times on chunks of a contiguous sequence will
	/// find matches the same way as if it was run on the whole sequence using [`scan`](StreamScanner::scan)
	pub fn scan_partial<I: Iterator<Item = u8>>(
		&mut self,
		offset: OffsetType,
		stream: I
	) -> StreamScannerIter<'_, P, I> {
		StreamScannerIter::new_partial(
			self,
			offset,
			stream
		)
	}

	/// Merges the candidates from other scanner into self.
	///
	/// This has the same effect as replaying the same partial scans that were run on `other` on self.
	pub fn merge_partial_mut(
		&mut self,
		mut other: Self
	) {
		self.candidates.append(
			&mut other.candidates
		);
	}

	// /// Resolves partial candidates left over by previous calls to [`scan_partial`](StreamScanner::scan_partial) or [`merge_partial_mut`](StreamScanner::merge_partial_mut).
	pub fn resolve_partial(
		&mut self
	) -> impl Iterator<Item = ScanResult> {
		let mut resolved = Vec::new();

		self.candidates.sort_unstable();
		AccFilter::acc_filter_vec_mut(
			&mut self.candidates,
			|acc, curr| {
				debug_assert!(
					!curr.is_resolved() || curr.is_partial()
				);
				match acc {
					None => acc.replace(curr),
					Some(a) => match a.try_merge_mut(curr) {
						Ok(()) => {
							// TODO: Did I just exploit my own api?
							if a.is_resolved() && !a.is_partial() {
								resolved.push(
									(a.offset(), a.length())
								);
								*acc = None;
							}

							None
						}
						Err(other) => acc.replace(other)
					}
				}
			}
		);

		resolved.into_iter()
	}
	
	fn on_start(&mut self, offset: OffsetType, byte: u8) {
		self.candidates.extend(self.predicate.try_start_partial_candidates(offset, byte));
	}
}

/// Iterator that runs scanner over the stream input.
///
/// This is constructed by [`scan_once`](StreamScanner::scan_once) and [`scan_partial`](StreamScanner::scan_partial).
pub struct StreamScannerIter<'a, P: ScannerPredicate, I: Iterator<Item = u8>> {
	scanner: &'a mut StreamScanner<P>,
	offset: OffsetType,
	stream: I,
	found: Vec<ScanResult>,
	found_yield_index: usize,
	reset_after: bool
}
impl<'a, P: ScannerPredicate, I: Iterator<Item = u8>> StreamScannerIter<'a, P, I> {
	pub fn new(
		scanner: &'a mut StreamScanner<P>,
		offset: OffsetType,
		stream: I
	) -> Self {
		StreamScannerIter {
			scanner,
			offset,
			stream,
			found: Vec::new(),
			found_yield_index: 0,
			reset_after: true
		}
	}

	fn get_buffered(&mut self) -> ScanResult {
		let result = self.found[self.found_yield_index];

		self.found_yield_index += 1;
		// if we've yielded all buffered results, reset the buffer
		if self.found_yield_index == self.found.len() {
			self.found.clear();
			self.found_yield_index = 0;
		}

		result
	}
}
impl<'a, P: PartialScannerPredicate, I: Iterator<Item = u8>> StreamScannerIter<'a, P, I> {
	pub fn new_partial(
		scanner: &'a mut StreamScanner<P>,
		offset: OffsetType,
		stream: I
	) -> Self {
		let mut stream = stream;

		// unroll the first iteration to run `on_start` here.
		let mut found = Vec::new();
		if let Some(first_byte) = stream.next() {
			scanner.on_start(offset, first_byte);
			scanner.on_byte(offset, first_byte, &mut found);
		}

		StreamScannerIter {
			scanner,
			offset: offset.saturating_add(1),
			stream,
			found,
			found_yield_index: 0,
			reset_after: false
		}
	}
}
impl<'a, P: ScannerPredicate, I: Iterator<Item = u8>> Iterator for StreamScannerIter<'a, P, I> {
	type Item = ScanResult;

	fn next(&mut self) -> Option<Self::Item> {
		// yield buffered results first
		if self.found_yield_index < self.found.len() {
			return Some(self.get_buffered());
		}

		// consume the stream until it either runs out or some results are generated
		let mut byte = self.stream.next();
		loop {
			match byte {
				// stream exhausted and no buffered results
				None => {
					if self.reset_after {
						self.scanner.reset();
					}

					return None
				}
				Some(byte) => {
					self.scanner.on_byte(
						self.offset,
						byte,
						&mut self.found
					);

					self.offset = self.offset.saturating_add(1);
				}
			}

			// loop until there are some results then yield the first
			if self.found.len() > 0 {
				return Some(self.get_buffered());
			}
			byte = self.stream.next();
		}
	}
}

#[cfg(test)]
mod test {
	use std::{convert::TryInto, num::NonZeroUsize};

	use super::StreamScanner;
	use crate::{common::AsRawBytes, predicate::value::ValuePredicate};

	#[test]
	fn test_stream_scanner() {
		let data = b"Hello There";

		let predicate = ValuePredicate::new(data, true);
		let mut scanner = StreamScanner::new(predicate);
		let found: Vec<_> = scanner.scan_once(
			1.into(), data.iter().copied()
		).collect();

		assert_eq!(found, &[(1.into(), NonZeroUsize::new(data.len()).unwrap())]);
	}

	#[test]
	fn test_stream_scanner_single_byte() {
		let data = 15u8;

		let predicate = ValuePredicate::new(data, true);
		let mut scanner = StreamScanner::new(predicate);
		let found: Vec<_> = scanner.scan_once(
			1.into(), std::iter::once(data)
		).collect();

		assert_eq!(found, &[(1.into(), NonZeroUsize::new(1).unwrap())]);
	}

	#[test]
	fn test_stream_scanner_multiple() {
		let data = [2u64, 1, 0, 1, 0, 1, 0, 0, 1, 0, 1, 0, 2];

		let predicate = ValuePredicate::new([1u64, 0, 1, 0], true);
		let mut scanner = StreamScanner::new(predicate);
		let found: Vec<_> = scanner.scan_once(
			8.into(),
			data.as_raw_bytes().iter().copied()
		).collect();

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
	fn test_stream_scanner_partial_multiple_pages_sorted() {
		let data = [2u64, 1, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0, 1];
		let second_data = [0u64, 1, 0];

		let predicate = ValuePredicate::new([1u64, 0, 1, 0], true);
		let mut scanner = StreamScanner::new(predicate);

		let mut found = Vec::new();
		found.extend(
			scanner.scan_partial(
				8.into(),
				data.as_raw_bytes().iter().copied()
			)
		);
		found.extend(
			scanner.scan_partial(
				112.into(),
				second_data.as_raw_bytes().iter().copied()
			)
		);

		assert_eq!(
			found,
			&[
				(16.into(), NonZeroUsize::new(32).unwrap()),
				(64.into(), NonZeroUsize::new(32).unwrap()),
				(104.into(), NonZeroUsize::new(32).unwrap()),
			]
		);
	}

	#[test]
	fn test_stream_scanner_partial_equals_once() {
		let data = [3u8, 4, 3, 4, 5, 6, 3, 4];
		let predicate = ValuePredicate::new([3u8, 4], true);

		let mut scanner = StreamScanner::new(predicate);

		let found_scan_once: Vec<_> = scanner.scan_once(1.into(), data.iter().copied()).collect();

		let mut found_scan_partial = Vec::new();
		found_scan_partial.extend(
			scanner.scan_partial(
				4.into(),
				data[3 ..].iter().copied()
			)
		);
		found_scan_partial.extend(
			scanner.scan_partial(
				1.into(),
				data[.. 3].iter().copied()
			)
		);
		found_scan_partial.extend(
			scanner.resolve_partial()
		);
		found_scan_partial.sort_unstable();

		assert_eq!(found_scan_once, found_scan_partial);
	}

	#[test]
	fn test_stream_scanner_partial_merge() {
		let data = [3u8, 4, 3, 4, 5, 6, 3, 4];
		let predicate = ValuePredicate::new([3u8, 4], true);

		let mut scanner_1 = StreamScanner::new(&predicate);
		let mut scanner_2 = StreamScanner::new(&predicate);
		let mut scanner_3 = StreamScanner::new(&predicate);

		let mut found_scan_partial = Vec::new();
		found_scan_partial.extend(
			scanner_1.scan_partial(
				4.into(),
				data[3 .. 7].iter().copied()
			)
		);
		found_scan_partial.extend(
			scanner_2.scan_partial(
				1.into(),
				data[.. 3].iter().copied()
			)
		);
		found_scan_partial.extend(
			scanner_3.scan_partial(
				8.into(),
				data[7 .. ].iter().copied()
			)
		);

		scanner_2.merge_partial_mut(scanner_3);
		scanner_1.merge_partial_mut(scanner_2);
		found_scan_partial.extend(
			scanner_1.resolve_partial()
		);
		found_scan_partial.sort_unstable();

		assert_eq!(
			found_scan_partial,
			&[
				(1.into(), NonZeroUsize::new(2).unwrap()),
				(3.into(), NonZeroUsize::new(2).unwrap()),
				(7.into(), NonZeroUsize::new(2).unwrap())
			]
		);
	}
}
