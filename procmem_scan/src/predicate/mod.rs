use procmem_access::prelude::OffsetType;

use crate::candidate::ScannerCandidate;

pub mod value;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum UpdateCandidateResult {
	/// Candidate is still valid, advance it and continue.
	Advance,
	/// Candidate is still valid but it should be skipped.
	Skip,
	/// Candidate is no longer valid, remove it from the candidate pool.
	Remove,
	/// Resolve the candidate into the match.
	Resolve
}

/// Scanner predicate is an interface which the scanner asks where
/// to create, update, delete and resolve candidates.
pub trait ScannerPredicate {
	/// Decides whether the currently read byte is a start of a candidate.
	fn try_start_candidate(&self, offset: OffsetType, byte: u8) -> Option<ScannerCandidate>;

	/// Decides whether the currently read byte is a valid continuation of the candidate.
	///
	/// This is only called of `offset == candidate.end_offset() + 1`.
	fn update_candidate(
		&self,
		offset: OffsetType,
		byte: u8,
		candidate: &ScannerCandidate
	) -> UpdateCandidateResult;
}
impl<T: ScannerPredicate, U: std::ops::Deref<Target = T>> ScannerPredicate for U {
	fn try_start_candidate(&self, offset: OffsetType, byte: u8) -> Option<ScannerCandidate> {
		(**self).try_start_candidate(offset, byte)
	}

	fn update_candidate(&self, offset: OffsetType, byte: u8, candidate: &ScannerCandidate) -> UpdateCandidateResult {
		(**self).update_candidate(offset, byte, candidate)
	}
}

/// Partial scanner predicate builds on scanner predicate and extends the interface with
/// partial candidate detection.
///
/// This allows the scanner to run on separate chunks of possibly out of order memory and
/// still detect matches across consecutive parts them as a whole.
///
/// One main usecase of this is running the scanner multi-threaded.
pub trait PartialScannerPredicate: ScannerPredicate {
	/// Decides whether the currently read byte is a start of any partial candidates.
	///
	/// This is only called at the very first byte of each scanned sequence.
	fn try_start_partial_candidates(&self, offset: OffsetType, byte: u8) -> Vec<ScannerCandidate>;
}
impl<T: PartialScannerPredicate, U: std::ops::Deref<Target = T>> PartialScannerPredicate for U {
	fn try_start_partial_candidates(&self, offset: OffsetType, byte: u8) -> Vec<ScannerCandidate> {
		(**self).try_start_partial_candidates(offset, byte)
	}
}