use std::num::NonZeroUsize;

use procmem_access::prelude::OffsetType;

use crate::{
	candidate::ScannerCandidate,
	common::AsRawBytes,
	predicate::{ScannerPredicate, UpdateCandidateResult}
};

use super::PartialScannerPredicate;

/// Predicate scanning for a concrete value in memory.
///
/// The value may be anything but is constrained to `AsRawBytes` because it needs to be accessed as raw bytes safely.
pub struct ValuePredicate<T: AsRawBytes> {
	value: T,
	aligned: bool
}
impl<T: AsRawBytes> ValuePredicate<T> {
	/// Creates a new predicate.
	///
	/// If `aligned` is true then candidates are only generated at offsets that are divisible by [`T::align_of`](AsRawBytes::align_of)
	pub fn new(value: T, aligned: bool) -> Self {
		debug_assert!(value.as_raw_bytes().len() > 0);

		ValuePredicate { value, aligned }
	}

	fn offset_aligned(&self, offset: OffsetType) -> bool {
		!self.aligned || (offset.get() % T::align_of()) == 0
	}
}
impl<T: AsRawBytes> ScannerPredicate for ValuePredicate<T> {
	fn try_start_candidate(&self, offset: OffsetType, byte: u8) -> Option<ScannerCandidate> {
		let bytes = self.value.as_raw_bytes();
		
		if self.offset_aligned(offset) {
			if bytes[0] == byte {
				let result = if bytes.len() == 1 {
					ScannerCandidate::resolved(offset, None)
				} else {
					ScannerCandidate::normal(offset)
				};

				return Some(result);
			}
		}

		None
	}

	fn update_candidate(
		&self,
		_offset: OffsetType,
		byte: u8,
		candidate: &ScannerCandidate
	) -> UpdateCandidateResult {
		let bytes = self.value.as_raw_bytes();
		debug_assert!(candidate.length().get() < bytes.len());

		if bytes[candidate.length().get()] != byte {
			return UpdateCandidateResult::Remove
		}

		if candidate.length().get() == bytes.len() - 1 {
			return UpdateCandidateResult::Resolve
		}

		UpdateCandidateResult::Advance
	}
}
impl<T: AsRawBytes> PartialScannerPredicate for ValuePredicate<T> {
	fn try_start_partial_candidates(&self, offset: OffsetType, byte: u8) -> Vec<ScannerCandidate> {
		let mut candidates = Vec::new();

		let bytes = self.value.as_raw_bytes();
		for (i, target_byte) in bytes
			.iter()
			.copied()
			.enumerate()
			.skip(1)
			.rev()
		{
			if byte != target_byte {
				continue
			}

			let potential_start_offset = match offset.get().checked_sub(i) {
				// skip this candidate if it would start at a non-positive offset
				// even though starting at offset 1 is also pretty unreal, it is not against our invariants
				None => continue,
				Some(p) if p == 0 => continue,
				Some(p) => p.into()
			};

			if !self.offset_aligned(potential_start_offset) {
				continue
			}

			let length = NonZeroUsize::new(i + 1).unwrap();
			let candidate = if length.get() == bytes.len() {
				ScannerCandidate::partial_resolved(potential_start_offset, length)
			} else {
				ScannerCandidate::partial(potential_start_offset, length)
			};

			candidates.push(candidate);
		}

		candidates
	}
}

#[cfg(test)]
mod test {
	use std::num::NonZeroUsize;

	use super::ValuePredicate;
	use crate::{candidate::ScannerCandidate, common::AsRawBytes, predicate::{ScannerPredicate, PartialScannerPredicate, UpdateCandidateResult}};

	#[test]
	fn test_value_predicate_start() {
		let data_u16 = [1u16];
		let data = data_u16.as_raw_bytes();

		let predicate = ValuePredicate::new([1], true);

		// Works correctly
		let result = predicate.try_start_candidate(100.into(), data[0]).unwrap();
		assert_eq!(result.offset(), 100.into());
		assert_eq!(result.start_offset(), 100.into());
		assert_eq!(result.length(), NonZeroUsize::new(1).unwrap());
		assert!(!result.is_partial());
		assert!(!result.is_resolved());

		// Rejects unaligned
		assert_eq!(predicate.try_start_candidate(101.into(), data[0]), None);
		// Rejects wrong start
		assert_eq!(predicate.try_start_candidate(100.into(), data[1]), None);
	}

	#[test]
	fn test_value_predicate_normal_length_1() {
		let data = 1u8;

		let predicate = ValuePredicate::new(1u8, false);

		let result = predicate.try_start_candidate(100.into(), data).unwrap();
		assert_eq!(result.offset(), 100.into());
		assert_eq!(result.start_offset(), 100.into());
		assert_eq!(result.length(), NonZeroUsize::new(1).unwrap());
		assert!(!result.is_partial());
		assert!(result.is_resolved());
	}

	#[test]
	fn test_value_predicate_partial_resolved() {
		let data = [1u8, 2, 3, 4];

		let predicate = ValuePredicate::new([2u8, 3], false);
		
		let result = predicate.try_start_partial_candidates(102.into(), data[2])[0];
		assert_eq!(result.offset(), 101.into());
		assert_eq!(result.start_offset(), 102.into());
		assert_eq!(result.length(), NonZeroUsize::new(2).unwrap());
		assert!(result.is_partial());
		assert!(result.is_resolved());
	}

	#[test]
	fn test_value_predicate_update() {
		let data_u16 = [1, std::u16::MAX];
		let data = unsafe {
			std::slice::from_raw_parts(
				&data_u16 as *const u16 as *const u8,
				data_u16.len() * std::mem::size_of::<u16>()
			)
		};

		let predicate = ValuePredicate::new([1, std::u16::MAX], true);

		// Works correctly
		assert_eq!(
			predicate.try_start_candidate(100.into(), data[0]),
			Some(ScannerCandidate::normal(100.into()))
		);
		let mut candidate = ScannerCandidate::normal(100.into());

		// valid continuation
		assert_eq!(
			predicate.update_candidate(101.into(), data[1], &candidate),
			UpdateCandidateResult::Advance
		);
		candidate.advance();

		// valid continuation
		assert_eq!(
			predicate.update_candidate(102.into(), data[2], &candidate),
			UpdateCandidateResult::Advance
		);
		candidate.advance();

		// final continuation
		assert_eq!(
			predicate.update_candidate(102.into(), data[3], &candidate),
			UpdateCandidateResult::Resolve
		);

		// invalid continuation
		assert_eq!(
			predicate.update_candidate(102.into(), data[1], &candidate),
			UpdateCandidateResult::Remove
		);
	}
}
