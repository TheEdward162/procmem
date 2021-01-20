//! Platform-agnostic memory scanning.

pub mod predicate;

pub mod candidate;
pub use candidate::ScannerCandidate;

pub mod sequential;
