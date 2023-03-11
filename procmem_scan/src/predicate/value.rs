use std::num::NonZeroUsize;

use procmem_access::prelude::OffsetType;

use crate::{
	candidate::ScannerCandidate,
	predicate::{ScannerPredicate, UpdateCandidateResult}
};

use super::PartialScannerPredicate;

pub trait ByteComparable {
	fn as_bytes(&self) -> &[u8];

	/// Returns the alignment requirement of the type.
	///
	/// This is needed for when the implementor of this trait is a reference.
	/// Then this function returns the alignment of the type behind reference, not of the reference itself.
	fn align_of() -> usize;
}
macro_rules! impl_byte_comparable {
	(
		Pod:
		$( $pod_type: ty )+
	) => {
		$(
			impl ByteComparable for $pod_type {
				fn as_bytes(&self) -> &[u8] {
					unsafe {
						std::slice::from_raw_parts(
							self as *const _ as *const u8,
							std::mem::size_of::<Self>()
						)
					}
				}
			
				fn align_of() -> usize {
					std::mem::align_of::<Self>()
				}
			}
			impl<const N: usize> ByteComparable for [$pod_type; N] {
				fn as_bytes(&self) -> &[u8] {
					unsafe {
						std::slice::from_raw_parts(
							self.as_slice().as_ptr() as *const u8,
							std::mem::size_of::<$pod_type>() * N
						)
					}
				}
			
				fn align_of() -> usize {
					<$pod_type as ByteComparable>::align_of()
				}
			}
			impl ByteComparable for &'_ [$pod_type] {
				fn as_bytes(&self) -> &[u8] {
					unsafe {
						std::slice::from_raw_parts(
							self.as_ptr() as *const u8,
							std::mem::size_of::<$pod_type>() * self.len()
						)
					}
				}
			
				fn align_of() -> usize {
					<$pod_type as ByteComparable>::align_of()
				}
			}
		)+
	};
}
impl_byte_comparable! {
	Pod: u8 i8 u16 i16 u32 i32 u64 i64 u128 i128 usize isize f32 f64
}
impl ByteComparable for &'_ str {
    fn as_bytes(&self) -> &[u8] {
        str::as_bytes(self)
    }

    fn align_of() -> usize {
        std::mem::align_of::<u8>()
    }
}

/// Predicate scanning for a concrete value in memory.
///
/// The value may be anything but is constrained to `ByteComparable` because it needs to be accessed as raw bytes safely.
pub struct ValuePredicate<T: ByteComparable> {
	value: T,
	aligned: bool
}
impl<T: ByteComparable> ValuePredicate<T> {
	/// Creates a new predicate.
	///
	/// If `aligned` is true then candidates are only generated at offsets that are divisible by [`T::align_of`](ByteComparable::align_of)
	pub fn new(value: T, aligned: bool) -> Self {
		debug_assert!(value.as_bytes().len() > 0);

		ValuePredicate { value, aligned }
	}

	fn offset_aligned(&self, offset: OffsetType) -> bool {
		!self.aligned || (offset.get() % T::align_of() as u64) == 0
	}
}
impl<T: ByteComparable> ScannerPredicate for ValuePredicate<T> {
	fn try_start_candidate(&self, offset: OffsetType, byte: u8) -> Option<ScannerCandidate> {
		let bytes = self.value.as_bytes();
		
		if self.offset_aligned(offset) {
			if bytes[0] == byte {
				let result = if bytes.len() == 1 {
					ScannerCandidate::resolved(offset, NonZeroUsize::new(1).unwrap())
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
		let bytes = self.value.as_bytes();
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
impl<T: ByteComparable> PartialScannerPredicate for ValuePredicate<T> {
	fn try_start_partial_candidates(&self, offset: OffsetType, byte: u8) -> Vec<ScannerCandidate> {
		let mut candidates = Vec::new();

		let bytes = self.value.as_bytes();
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

			let potential_start_offset = match offset.get().saturating_sub(i as u64) {
				// skip this candidate if it would start at a non-positive offset
				// even though starting at offset 1 is also pretty unreal, it is not against our invariants
				0 => continue,
				p => OffsetType::new_unwrap(p)
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

	use procmem_access::prelude::OffsetType;

    use super::ValuePredicate;
	use crate::{
		candidate::ScannerCandidate,
		predicate::{ScannerPredicate, PartialScannerPredicate, UpdateCandidateResult, value::ByteComparable}
	};

	#[test]
	fn test_value_predicate_start() {
		let data_u16 = [1u16];
		let data = data_u16.as_bytes();

		let predicate = ValuePredicate::new([1], true);

		// Works correctly
		let result = predicate.try_start_candidate(OffsetType::new_unwrap(100), data[0]).unwrap();
		assert_eq!(result.offset(), OffsetType::new_unwrap(100));
		assert_eq!(result.start_offset(), OffsetType::new_unwrap(100));
		assert_eq!(result.length(), NonZeroUsize::new(1).unwrap());
		assert!(!result.is_partial());
		assert!(!result.is_resolved());

		// Rejects unaligned
		assert_eq!(predicate.try_start_candidate(OffsetType::new_unwrap(101), data[0]), None);
		// Rejects wrong start
		assert_eq!(predicate.try_start_candidate(OffsetType::new_unwrap(100), data[1]), None);
	}

	#[test]
	fn test_value_predicate_normal_length_1() {
		let data = 1u8;

		let predicate = ValuePredicate::new(1u8, false);

		let result = predicate.try_start_candidate(OffsetType::new_unwrap(100), data).unwrap();
		assert_eq!(result.offset(), OffsetType::new_unwrap(100));
		assert_eq!(result.start_offset(), OffsetType::new_unwrap(100));
		assert_eq!(result.length(), NonZeroUsize::new(1).unwrap());
		assert!(!result.is_partial());
		assert!(result.is_resolved());
	}

	#[test]
	fn test_value_predicate_partial_resolved() {
		let data = [1u8, 2, 3, 4];

		let predicate = ValuePredicate::new([2u8, 3], false);
		
		let result = predicate.try_start_partial_candidates(OffsetType::new_unwrap(102), data[2])[0];
		assert_eq!(result.offset(), OffsetType::new_unwrap(101));
		assert_eq!(result.start_offset(), OffsetType::new_unwrap(102));
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
			predicate.try_start_candidate(OffsetType::new_unwrap(100), data[0]),
			Some(ScannerCandidate::normal(OffsetType::new_unwrap(100)))
		);
		let mut candidate = ScannerCandidate::normal(OffsetType::new_unwrap(100));

		// valid continuation
		assert_eq!(
			predicate.update_candidate(OffsetType::new_unwrap(101), data[1], &candidate),
			UpdateCandidateResult::Advance
		);
		candidate.advance();

		// valid continuation
		assert_eq!(
			predicate.update_candidate(OffsetType::new_unwrap(102), data[2], &candidate),
			UpdateCandidateResult::Advance
		);
		candidate.advance();

		// final continuation
		assert_eq!(
			predicate.update_candidate(OffsetType::new_unwrap(102), data[3], &candidate),
			UpdateCandidateResult::Resolve
		);

		// invalid continuation
		assert_eq!(
			predicate.update_candidate(OffsetType::new_unwrap(102), data[1], &candidate),
			UpdateCandidateResult::Remove
		);
	}
}
