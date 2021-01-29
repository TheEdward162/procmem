use std::{
	cmp::{Ord, Ordering, PartialOrd},
	num::NonZeroUsize
};

use procmem_access::{prelude::OffsetType, util::AccFilter};

/// Candidate match for stream scanner.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ScannerCandidate {
	/// Offset where the value match starts.
	offset: OffsetType,
	/// Length of the candidate match.
	length: NonZeroUsize,
	/// Whether the candidate has been resolved.
	resolved: bool,
	/// Offset at which this (partial) candidate starts, if different from `offset`.
	///
	/// This value is always greated than `start_offset`.
	start_offset: Option<OffsetType>
}
impl ScannerCandidate {
	pub fn normal(offset: OffsetType) -> Self {
		ScannerCandidate {
			offset,
			length: NonZeroUsize::new(1).unwrap(),
			resolved: false,
			start_offset: None
		}
	}

	/// Creates a new instance of scanner candidate that describes a partial candidate.
	///
	/// `length` is the length of the would-be match from `offset` to where the match was found.
	pub fn partial(offset: OffsetType, length: NonZeroUsize) -> Self {
		ScannerCandidate {
			offset,
			length,
			resolved: false,
			start_offset: Some(
				OffsetType::new_unwrap(offset.get() + length.get() as u64 - 1)
			)
		}
	}

	/// Creates a new instance of scanner candidate that describes an already resolved candidate.
	///
	/// If `length` is None, it is defaulted to one.
	pub fn resolved(offset: OffsetType, length: Option<NonZeroUsize>) -> Self {
		ScannerCandidate {
			offset,
			length: length.unwrap_or(NonZeroUsize::new(1).unwrap()),
			resolved: true,
			start_offset: None
		}
	}

	/// Creates a new instance of scanner candidate that describes a partial, resolved candidate.
	///
	/// The parameters behave the same as with [`partial`](ScannerCadidate::partial).
	pub fn partial_resolved(offset: OffsetType, length: NonZeroUsize) -> Self {
		Self {
			resolved: true,
			.. Self::partial(offset, length)
		}
	}

	pub const fn is_partial(&self) -> bool {
		self.start_offset.is_some()
	}

	pub const fn is_resolved(&self) -> bool {
		self.resolved
	}

	/// Returns the offset where the value match starts.
	///
	/// For partial matches, this returns the offset where the match should start, not where it was found.
	pub const fn offset(&self) -> OffsetType {
		self.offset
	}

	/// Returns the length of the match since [`offset`](ScannerCandidate::offset).
	pub const fn length(&self) -> NonZeroUsize {
		self.length
	}

	/// Offset where the definitely matched candidate starts.
	///
	/// This differs from [`offset`](ScannerCandidate::offset) only for partial candidates.
	pub fn start_offset(&self) -> OffsetType {
		self.start_offset.unwrap_or(self.offset)
	}

	pub const fn end_offset(&self) -> OffsetType {
		self.offset().saturating_add(self.length().get() as u64)
	}

	/// Advances the candidate (increases the length).
	pub fn advance(&mut self) {
		debug_assert!(!self.resolved);

		unsafe {
			self.length = NonZeroUsize::new_unchecked(self.length.get() + 1);
		}
	}

	/// Resolved this candidate by advancing it one last time and setting the resolved flag.
	pub fn resolve(&mut self) {
		self.advance();
		self.resolved = true;
	}

	/// Attempts to merge two candidates in place.
	///
	/// Returns `Err(other)` if they cannot be merged.
	///
	/// Two candidates can be merged if they overlap in the range they definitely match.
	///
	/// **Note:** This method does not and cannot differentiate between candidates that
	/// do not come from the same predicate. Thus the result of this merge may not be
	/// logically valid if it is not run on two candidates coming from the same predicate.
	pub fn try_merge_mut(&mut self, other: Self) -> Result<(), Self> {
		// Cannot be the same match if they don't start in the same place
		if self.offset() != other.offset() {
			return Err(other)
		}

		// Cannot merge if they don't intersect.
		if self.end_offset() < other.start_offset() || other.end_offset() < self.start_offset() {
			return Err(other)
		}

		self.length = self.length.max(other.length);
		self.resolved = self.resolved || other.resolved;
		self.start_offset = self.start_offset.min(other.start_offset);

		Ok(())
	}

	/// Returns an adapted iterator that will merge all consecutive candidates in the iterator using [`try_merge_mut`](ScannerCandidate::try_merge_mut).
	pub fn merge_sorted(iter: impl Iterator<Item = Self>) -> impl Iterator<Item = Self> {
		AccFilter::new(iter, |acc, curr| match acc {
			None => acc.replace(curr),
			Some(a) => match a.try_merge_mut(curr) {
				Ok(()) => None,
				Err(other) => acc.replace(other)
			}
		})
	}
}
impl PartialOrd for ScannerCandidate {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(&other))
	}
}
impl Ord for ScannerCandidate {
	fn cmp(&self, other: &Self) -> Ordering {
		self.offset()
			.cmp(&other.offset())
			.then(self.start_offset.cmp(&other.start_offset))
			.then(self.length().cmp(&other.length()))
	}
}

#[cfg(test)]
mod test {
	use std::num::NonZeroUsize;

	use procmem_access::prelude::OffsetType;

    use super::ScannerCandidate;

	#[test]
	fn test_scanner_candidate_construction() {
		let candidate = ScannerCandidate::normal(OffsetType::new_unwrap(10));
		assert_eq!(
			candidate,
			ScannerCandidate {
				offset: OffsetType::new_unwrap(10),
				length: NonZeroUsize::new(1).unwrap(),
				resolved: false,
				start_offset: None
			}
		);

		let candidate = ScannerCandidate::resolved(OffsetType::new_unwrap(20), NonZeroUsize::new(12));
		assert_eq!(
			candidate,
			ScannerCandidate {
				offset: OffsetType::new_unwrap(20),
				length: NonZeroUsize::new(12).unwrap(),
				resolved: true,
				start_offset: None
			}
		);

		let candidate = ScannerCandidate::partial(OffsetType::new_unwrap(11), NonZeroUsize::new(5).unwrap());
		assert_eq!(
			candidate,
			ScannerCandidate {
				offset: OffsetType::new_unwrap(11),
				length: NonZeroUsize::new(5).unwrap(),
				resolved: false,
				start_offset: Some(OffsetType::new_unwrap(15))
			}
		);

		let candidate = ScannerCandidate::partial_resolved(OffsetType::new_unwrap(10), NonZeroUsize::new(2).unwrap());
		assert_eq!(
			candidate,
			ScannerCandidate {
				offset: OffsetType::new_unwrap(10),
				length: NonZeroUsize::new(2).unwrap(),
				resolved: true,
				start_offset: Some(OffsetType::new_unwrap(11))
			}
		);
	}

	#[test]
	fn test_scanner_candidate_sort() {
		let mut candidates = [
			ScannerCandidate {
				offset: OffsetType::new_unwrap(2),
				length: NonZeroUsize::new(2).unwrap(),
				resolved: false,
				start_offset: None
			},
			ScannerCandidate {
				offset: OffsetType::new_unwrap(2),
				length: NonZeroUsize::new(1).unwrap(),
				resolved: false,
				start_offset: None
			},
			ScannerCandidate {
				offset: OffsetType::new_unwrap(1),
				length: NonZeroUsize::new(3).unwrap(),
				resolved: false,
				start_offset: Some(OffsetType::new_unwrap(1))
			},
			ScannerCandidate {
				offset: OffsetType::new_unwrap(1),
				length: NonZeroUsize::new(2).unwrap(),
				resolved: false,
				start_offset: None
			}
		];

		candidates.sort();

		assert_eq!(
			candidates,
			[
				ScannerCandidate {
					offset: OffsetType::new_unwrap(1),
					length: NonZeroUsize::new(2).unwrap(),
					resolved: false,
					start_offset: None
				},
				ScannerCandidate {
					offset: OffsetType::new_unwrap(1),
					length: NonZeroUsize::new(3).unwrap(),
					resolved: false,
					start_offset: Some(OffsetType::new_unwrap(1))
				},
				ScannerCandidate {
					offset: OffsetType::new_unwrap(2),
					length: NonZeroUsize::new(1).unwrap(),
					resolved: false,
					start_offset: None
				},
				ScannerCandidate {
					offset: OffsetType::new_unwrap(2),
					length: NonZeroUsize::new(2).unwrap(),
					resolved: false,
					start_offset: None
				}
			]
		);
	}

	#[test]
	fn test_scanner_candidate_merge() {
		let values = [
			ScannerCandidate {
				offset: OffsetType::new_unwrap(1),
				length: NonZeroUsize::new(2).unwrap(),
				resolved: false,
				start_offset: None
			},
			ScannerCandidate {
				offset: OffsetType::new_unwrap(1),
				length: NonZeroUsize::new(3).unwrap(),
				resolved: false,
				start_offset: Some(OffsetType::new_unwrap(1))
			},
			ScannerCandidate {
				offset: OffsetType::new_unwrap(2),
				length: NonZeroUsize::new(1).unwrap(),
				resolved: false,
				start_offset: None
			},
			ScannerCandidate {
				offset: OffsetType::new_unwrap(2),
				length: NonZeroUsize::new(2).unwrap(),
				resolved: true,
				start_offset: None
			}
		];

		let result = ScannerCandidate::merge_sorted(values.iter().copied()).collect::<Vec<_>>();

		assert_eq!(
			result,
			&[
				ScannerCandidate {
					offset: OffsetType::new_unwrap(1),
					length: NonZeroUsize::new(3).unwrap(),
					resolved: false,
					start_offset: None
				},
				ScannerCandidate {
					offset: OffsetType::new_unwrap(2),
					length: NonZeroUsize::new(2).unwrap(),
					resolved: true,
					start_offset: None
				}
			]
		);
	}

	#[test]
	fn test_scanner_candidate_merge_start() {
		// 8  9  10  11
		// 1  2   3   4
		// ^------^ left
		//            ^ right
		let mut left = ScannerCandidate {
			offset: OffsetType::new_unwrap(8),
			length: NonZeroUsize::new(3).unwrap(),
			resolved: false,
			start_offset: None
		};
		let right = ScannerCandidate {
			offset: OffsetType::new_unwrap(8),
			length: NonZeroUsize::new(4).unwrap(),
			resolved: false,
			start_offset: Some(OffsetType::new_unwrap(10))
		};

		left.try_merge_mut(right).unwrap();

		assert_eq!(
			left,
			ScannerCandidate {
				offset: OffsetType::new_unwrap(8),
				length: NonZeroUsize::new(4).unwrap(),
				resolved: false,
				start_offset: None
			}
		);
	}

	#[test]
	fn test_scanner_candidate_merge_middle() {
		// 8  9  10  11
		// 1  2   3   4
		//    ^---^ left
		//            ^ right
		let mut left = ScannerCandidate {
			offset: OffsetType::new_unwrap(8),
			length: NonZeroUsize::new(3).unwrap(),
			resolved: false,
			start_offset: Some(OffsetType::new_unwrap(9))
		};
		let right = ScannerCandidate {
			offset: OffsetType::new_unwrap(8),
			length: NonZeroUsize::new(4).unwrap(),
			resolved: true,
			start_offset: Some(OffsetType::new_unwrap(11))
		};

		left.try_merge_mut(right).unwrap();

		assert_eq!(
			left,
			ScannerCandidate {
				offset: OffsetType::new_unwrap(8),
				length: NonZeroUsize::new(4).unwrap(),
				resolved: true,
				start_offset: Some(OffsetType::new_unwrap(9))
			}
		);
	}

	#[test]
	fn test_scanner_candidate_merge_end() {
		// 8  9  10  11
		// 1  2   3   4
		//    ^ left
		//        ^---^ right
		let mut left = ScannerCandidate {
			offset: OffsetType::new_unwrap(8),
			length: NonZeroUsize::new(2).unwrap(),
			resolved: false,
			start_offset: Some(OffsetType::new_unwrap(9))
		};
		let right = ScannerCandidate {
			offset: OffsetType::new_unwrap(8),
			length: NonZeroUsize::new(4).unwrap(),
			resolved: true,
			start_offset: Some(OffsetType::new_unwrap(10))
		};

		left.try_merge_mut(right).unwrap();

		assert_eq!(
			left,
			ScannerCandidate {
				offset: OffsetType::new_unwrap(8),
				length: NonZeroUsize::new(4).unwrap(),
				resolved: true,
				start_offset: Some(OffsetType::new_unwrap(9))
			}
		);
	}

	#[test]
	fn test_scanner_candidate_merge_err() {
		let mut left = ScannerCandidate {
			offset: OffsetType::new_unwrap(9),
			length: NonZeroUsize::new(2).unwrap(),
			resolved: false,
			start_offset: Some(OffsetType::new_unwrap(10))
		};
		let right = ScannerCandidate {
			offset: OffsetType::new_unwrap(8),
			length: NonZeroUsize::new(4).unwrap(),
			resolved: true,
			start_offset: Some(OffsetType::new_unwrap(12))
		};
		left.try_merge_mut(right).unwrap_err();
		assert_eq!(left.length.get(), 2);

		let mut left = ScannerCandidate {
			offset: OffsetType::new_unwrap(8),
			length: NonZeroUsize::new(2).unwrap(),
			resolved: false,
			start_offset: None
		};
		let right = ScannerCandidate {
			offset: OffsetType::new_unwrap(8),
			length: NonZeroUsize::new(4).unwrap(),
			resolved: true,
			start_offset: Some(OffsetType::new_unwrap(12))
		};
		left.try_merge_mut(right).unwrap_err();
		assert_eq!(left.length.get(), 2);
	}
}
