use crate::{
	scan::ScanPrimitiveType,
	util::{merge::MergeIter, OffsetType}
};

use super::{
	super::{ScanEntry, ScanFlow},
	ScanCallback
};

#[derive(Debug, PartialEq, Eq)]
struct Candidate {
	/// Offset of the start of the array.
	offset: OffsetType,
	/// Position in the target array until which this candidate matches.
	position: usize,
	/// If set, this is the offset at which the partial candidate begins,
	/// `self.offset` is a calculated value that might not be actually memory mapped.
	partial_offset: Option<OffsetType>
}
impl Candidate {
	pub fn new(offset: OffsetType, position: usize) -> Self {
		Candidate {
			offset,
			position,
			partial_offset: None
		}
	}

	pub fn partial(partial_offset: OffsetType, start_position: usize) -> Self {
		Candidate {
			offset: partial_offset.get().saturating_sub(start_position).into(),
			position: start_position,
			partial_offset: Some(partial_offset)
		}
	}

	/// Attempts to merge two candidates in place.
	///
	/// Assumes `self <= other`
	///
	/// Candidates are merged if both of them are partial and
	/// `self` ends where `other` begins.
	///
	/// Returns `Err(right)` if they cannot be merged.
	pub fn try_merge(&mut self, right: Self) -> Result<(), Self> {
		// Both have to start in the same place
		if self.offset != right.offset {
			return Err(right)
		}

		// Both have to be partial
		let right_start = match (self.partial_offset, right.partial_offset) {
			(Some(_), Some(o)) => o,
			_ => return Err(right)
		};
		let left_end = self.end_offset();

		// Left has to end where right begins
		if left_end.get() + 1 != right_start.get() {
			return Err(right)
		}

		self.position = right.position;

		Ok(())
	}

	pub fn partial_len(&self) -> Option<usize> {
		match self.partial_offset {
			None => None,
			Some(p) => Some(self.position + 1 - (self.offset.get() - p.get()))
		}
	}

	pub fn end_offset(&self) -> OffsetType {
		self.offset.saturating_add(self.position)
	}
}
impl std::cmp::PartialOrd for Candidate {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(&other))
	}
}
impl std::cmp::Ord for Candidate {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.offset
			.cmp(&other.offset)
			.then(self.position.cmp(&other.position))
			.then(self.partial_offset.cmp(&other.partial_offset))
	}
}

#[derive(Debug)]
pub struct ArrayFinder<T: ScanPrimitiveType, A: AsRef<[T]>> {
	target: A,
	candidates: Vec<Candidate>,
	found: Vec<OffsetType>,
	_boo: std::marker::PhantomData<T>
}
impl<T: ScanPrimitiveType, A: AsRef<[T]>> ArrayFinder<T, A> {
	pub fn new(target: A) -> Self {
		debug_assert!(target.as_ref().len() > 1);

		ArrayFinder {
			target,
			candidates: Vec::new(),
			found: Vec::new(),
			_boo: std::marker::PhantomData
		}
	}

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

	fn on_entry(&mut self, offset: OffsetType, element: T) {
		let target = self.target.as_ref();

		// go over candidate entries
		// if the entry fits, update it
		// if it fails, remove it
		let mut i = 0;
		while i < self.candidates.len() {
			let remove = {
				let mut candidate = &mut self.candidates[i];

				if candidate.end_offset().get() + 1 != offset.get() {
					// keep the candidate, this is a different offset
					// TODO: although assuming entries always come in order, this should be a true
					false
				} else if candidate.position == target.len() - 1 {
					debug_assert!(candidate.partial_offset.is_some());
					// keep the candidate, it is partial
					false
				} else {
					candidate.position += 1;

					if element != target[candidate.position] {
						// candidate turned out to not match
						true
					} else if candidate.position != target.len() - 1 {
						// keep the candidate, position matches
						false
					} else if candidate.partial_offset.is_some() {
						// keep the candidate, it is partial
						false
					} else {
						self.found.push(candidate.offset);
						// remove the candidate because it has now been found
						true
					}
				}
			};

			if remove {
				self.candidates.remove(i);
			} else {
				i += 1;
			}
		}

		// add new entry if the start matches
		if target[0] == element {
			self.candidates.push(Candidate::new(offset, 0));
		}
	}

	fn on_page_start(&mut self, offset: OffsetType, element: T) {
		for (i, t) in self.target.as_ref().iter().enumerate().skip(1) {
			if t == &element {
				self.candidates.push(Candidate::partial(offset, i));
			}
		}
	}

	fn on_page_end(&mut self, offset: OffsetType) {
		// TODO: Oh unstable, why must you hurt me so
		// self.candidates.drain_filter(filter)

		// remove all candidates that aren't partial and don't end at the page boundary
		let mut i = 0;
		while i < self.candidates.len() {
			let remove = {
				let mut candidate = &mut self.candidates[i];

				// don't remove partial candidates
				if candidate.partial_offset.is_some() {
					false
				} else {
					if candidate.end_offset() == offset {
						candidate.partial_offset = Some(candidate.offset);
						// don't remove, ends at page boundary
						false
					} else {
						// remove all other candidates
						true
					}
				}
			};

			if remove {
				self.candidates.remove(i);
			} else {
				i += 1;
			}
		}
	}

	/// Returns a slice of offsets at which arrays have been found.
	pub fn found(&self) -> &[OffsetType] {
		&self.found
	}
}
impl<T: ScanPrimitiveType, A: AsRef<[T]>> ScanCallback for ArrayFinder<T, A> {
	fn entry(&mut self, entry: ScanEntry) -> ScanFlow {
		if let Some(element) = entry.data.try_cast::<T>() {
			self.on_entry(entry.offset, element);
		}

		ScanFlow::Continue
	}

	fn page_start(&mut self, entry: ScanEntry) -> ScanFlow {
		if let Some(element) = entry.data.try_cast::<T>() {
			self.on_page_start(entry.offset, element);
		}

		ScanFlow::Continue
	}

	fn page_end(&mut self, offset: crate::util::OffsetType) {
		self.on_page_end(offset);
	}
}

#[cfg(test)]
mod test {
	use super::{ArrayFinder, Candidate};
	use crate::scan::{callback::ScanCallback, ScanEntry, ScanFlow, ScanPrimitiveType};

	#[test]
	fn test_array_candidate_merge() {
		let mut left = Candidate {
			offset: 10.into(),
			position: 1,
			partial_offset: Some(10.into())
		};
		let right = Candidate {
			offset: 10.into(),
			position: 3,
			partial_offset: Some(12.into())
		};

		left.try_merge(right).unwrap();

		assert_eq!(left.position, 3);
	}

	#[test]
	fn test_array_candidate_merge_err() {
		let mut left = Candidate {
			offset: 11.into(),
			position: 1,
			partial_offset: Some(10.into())
		};
		let right = Candidate {
			offset: 10.into(),
			position: 3,
			partial_offset: Some(12.into())
		};
		left.try_merge(right).unwrap_err();
		assert_eq!(left.position, 1);

		let mut left = Candidate {
			offset: 10.into(),
			position: 1,
			partial_offset: Some(10.into())
		};
		let right = Candidate {
			offset: 10.into(),
			position: 3,
			partial_offset: Some(13.into())
		};
		left.try_merge(right).unwrap_err();
		assert_eq!(left.position, 1);
	}

	#[test]
	fn test_array_finder() {
		let value = b"Hello There";

		let mut finder = ArrayFinder::new(value);

		for (i, &byte) in value.into_iter().enumerate() {
			let res = finder.entry(ScanEntry::u8((i + 10).into(), byte));
			assert_eq!(res, ScanFlow::Continue);
		}

		assert_eq!(finder.found(), &[10.into()]);
	}

	#[test]
	fn test_array_finder_multiple() {
		let data = [2u64, 1, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0, 1];

		let mut finder = ArrayFinder::new([1u64, 0, 1, 0]);

		for (i, &value) in data.iter().enumerate() {
			let res = finder.entry(ScanEntry::u64((i + 10).into(), value));
			assert_eq!(res, ScanFlow::Continue);
		}
		assert_eq!(finder.found(), &[11.into(), 17.into()]);
	}

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
}
