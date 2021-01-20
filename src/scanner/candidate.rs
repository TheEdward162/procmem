use std::num::NonZeroUsize;

use crate::common::OffsetType;

#[derive(Debug, PartialEq, Eq)]
pub struct ScannerCandidate {
	/// Offset at which this candidate starts.
	///
	/// For partial candidates, this is where the match would start.
	offset: OffsetType,
	/// Number of bytes for which this candidate matches.
	length: NonZeroUsize,
	/// If set, this is the offset at which this partial match was started.
	partial_offset: Option<OffsetType>
}
impl ScannerCandidate {
	pub fn new(offset: OffsetType) -> Self {
		ScannerCandidate {
			offset,
			length: NonZeroUsize::new(1).unwrap(),
			partial_offset: None
		}
	}

	/// Creates a partial candidate found at `partial_offset` which 
	/// corresponds to `length_here` within target.
	pub fn partial(partial_offset: OffsetType, length_here: NonZeroUsize) -> Self {
		ScannerCandidate {
			offset: partial_offset.get().saturating_sub(length_here.get()).into(),
			length: length_here,
			partial_offset: Some(partial_offset)
		}
	}

	pub fn advance(&mut self) {
		unsafe {
			self.length = NonZeroUsize::new_unchecked(
				self.length.get() + 1
			);
		}
	}

	pub fn is_partial(&self) -> bool {
		self.partial_offset.is_some()
	}

	/// If partial, returns the length of this candidate since the partial offset.
	pub fn partial_length(&self) -> Option<NonZeroUsize> {
		self.partial_offset.map(
			// TODO: this is safe to use `new_unchecked`, right?
			|partial_offset| NonZeroUsize::new(
				self.length.get() - (partial_offset.get() - self.offset.get())
			).unwrap()
		)
	}

	pub fn end_offset(&self) -> OffsetType {
		self.offset.saturating_add(self.length.get())
	}

	/// Attempts to merge two candidates in place.
	///
	/// Assumes `self <= other`
	///
	/// Candidates are merged if both of them are partial and
	/// `self` ends where `other` begins.
	///
	/// Returns `Err(right)` if they cannot be merged.
	pub fn try_merge_mut(&mut self, right: Self) -> Result<(), Self> {
		// Both have to start in the same place
		if self.offset != right.offset {
			return Err(right)
		}

		debug_assert!(*self <= right);

		// Both have to be partial
		let right_start = match (self.partial_offset, right.partial_offset) {
			(Some(_), Some(o)) => o,
			_ => return Err(right)
		};
		let left_end = self.end_offset();

		// Left has to end where right begins
		if left_end.get() != right_start.get() {
			return Err(right)
		}

		self.length = right.length;

		Ok(())
	}

	pub const fn offset(&self) -> OffsetType {
		self.offset
	} 

	pub const fn length(&self) -> NonZeroUsize {
		self.length
	}
}
impl std::cmp::PartialOrd for ScannerCandidate {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(&other))
	}
}
impl std::cmp::Ord for ScannerCandidate {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.offset.cmp(&other.offset)
			.then(self.length.cmp(&other.length))
			.then(self.partial_offset.cmp(&other.partial_offset))
	}
}

#[cfg(test)]
mod test {
	use std::convert::TryInto;

	use super::ScannerCandidate;

	#[test]
	fn test_scanner_candidate_merge() {
		let mut left = ScannerCandidate {
			offset: 10.into(),
			length: 2.try_into().unwrap(),
			partial_offset: Some(10.into())
		};
		let right = ScannerCandidate {
			offset: 10.into(),
			length: 4.try_into().unwrap(),
			partial_offset: Some(12.into())
		};

		left.try_merge_mut(right).unwrap();

		assert_eq!(left.length.get(), 4);
	}

	#[test]
	fn test_scanner_candidate_merge_err() {
		let mut left = ScannerCandidate {
			offset: 11.into(),
			length: 2.try_into().unwrap(),
			partial_offset: Some(10.into())
		};
		let right = ScannerCandidate {
			offset: 10.into(),
			length: 4.try_into().unwrap(),
			partial_offset: Some(12.into())
		};
		left.try_merge_mut(right).unwrap_err();
		assert_eq!(left.length.get(), 2);

		let mut left = ScannerCandidate {
			offset: 10.into(),
			length: 2.try_into().unwrap(),
			partial_offset: Some(10.into())
		};
		let right = ScannerCandidate {
			offset: 10.into(),
			length: 4.try_into().unwrap(),
			partial_offset: Some(13.into())
		};
		left.try_merge_mut(right).unwrap_err();
		assert_eq!(left.length.get(), 2);
	}
}