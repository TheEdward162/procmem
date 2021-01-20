use crate::common::{OffsetType, AsRawBytes};

use crate::scanner::predicate::{ScannerPredicate, UpdateCandidateResult};
use crate::scanner::candidate::ScannerCandidate;

pub struct ValuePredicate<T: AsRawBytes> {
	value: T,
	aligned: bool
}
impl<T: AsRawBytes> ValuePredicate<T> {
	pub fn new(value: T, aligned: bool) -> Self {
		ValuePredicate {
			value,
			aligned
		}
	}
}
impl<T: AsRawBytes> ScannerPredicate for ValuePredicate<T> {
	fn try_start_candidate(&self, offset: OffsetType, byte: u8) -> Option<ScannerCandidate> {
		if !self.aligned || (offset.get() % T::align_of()) == 0 {
			if self.value.as_raw_bytes()[0] == byte {
				return Some(
					ScannerCandidate::new(offset)
				);
			}
		}

		None
	}

	fn update_candidate(&self, _offset: OffsetType, byte: u8, candidate: &ScannerCandidate) -> UpdateCandidateResult {
		let bytes = self.value.as_raw_bytes();
		debug_assert!(candidate.length().get() < bytes.len());

		if bytes[candidate.length().get()] != byte {
			return UpdateCandidateResult::Remove;
		}

		if candidate.length().get() == bytes.len() - 1 {
			return UpdateCandidateResult::Resolve;
		}

		UpdateCandidateResult::Advance
	}
}

#[cfg(test)]
mod test {
	use super::ValuePredicate;
	use crate::scanner::predicate::{ScannerPredicate, UpdateCandidateResult};
	use crate::scanner::candidate::ScannerCandidate;

	#[test]
	fn test_value_predicate_start() {
		let data_u16 = [1u16];
		let data = unsafe {
			std::slice::from_raw_parts(
				&data_u16 as *const u16 as *const u8,
				data_u16.len() * std::mem::size_of::<u16>()
			)
		};
		
		let predicate = ValuePredicate::new(
			[1],
			true
		);

		// Works correctly
		assert_eq!(
			predicate.try_start_candidate(100.into(), data[0]),
			Some(ScannerCandidate::new(100.into()))
		);
		// Rejects unaligned
		assert_eq!(
			predicate.try_start_candidate(101.into(), data[0]),
			None
		);
		// Rejects wrong start
		assert_eq!(
			predicate.try_start_candidate(100.into(), data[1]),
			None
		);
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
		
		let predicate = ValuePredicate::new(
			[1, std::u16::MAX],
			true
		);

		// Works correctly
		assert_eq!(
			predicate.try_start_candidate(100.into(), data[0]),
			Some(ScannerCandidate::new(100.into()))
		);
		let mut candidate = ScannerCandidate::new(100.into());
		
		// valid continuation
		assert_eq!(
			predicate.update_candidate(
				101.into(),
				data[1],
				&candidate
			),
			UpdateCandidateResult::Advance
		);
		candidate.advance();

		// valid continuation
		assert_eq!(
			predicate.update_candidate(
				102.into(),
				data[2],
				&candidate
			),
			UpdateCandidateResult::Advance
		);
		candidate.advance();

		// final continuation
		assert_eq!(
			predicate.update_candidate(
				102.into(),
				data[3],
				&candidate
			),
			UpdateCandidateResult::Resolve
		);

		// invalid continuation
		assert_eq!(
			predicate.update_candidate(
				102.into(),
				data[1],
				&candidate
			),
			UpdateCandidateResult::Remove
		);
	}
}