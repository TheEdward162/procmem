use std::any::TypeId;
use std::ops::Deref;

pub mod base;
pub mod scanner;
pub mod callback;

/// Trait for types that appear in scan entries.
///
/// ## Safety
/// This trait is unsafe because `ByteScanner` relies on valid memory representation of these values
pub unsafe trait ScanPrimitiveType: 'static + Sized + PartialEq + Copy + std::fmt::Debug {
	fn try_cast<T: 'static + Sized>(value: T) -> Option<Self> {
		if TypeId::of::<T>() == TypeId::of::<Self>() {
			let fragile = std::mem::ManuallyDrop::new(value);
			
			// This is safe because we just checked that the TypeId of T and Self are equal => they are the same type
			let value: Self = unsafe {
				std::ptr::read(fragile.deref() as *const T as *const Self)
			};
			
			Some(
				value
			)
		} else {
			None
		}
	}
}
unsafe impl ScanPrimitiveType for u8 {}
unsafe impl ScanPrimitiveType for u16 {}
unsafe impl ScanPrimitiveType for u32 {}
unsafe impl ScanPrimitiveType for u64 {}
unsafe impl ScanPrimitiveType for usize {}
unsafe impl ScanPrimitiveType for f32 {}
unsafe impl ScanPrimitiveType for f64 {}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ScanEntryData {
	u8(u8),
	u16(u16),
	u32(u32),
	u64(u64),
	usize(usize),
	f32(f32),
	f64(f64)
}
impl ScanEntryData {
	pub fn try_cast<T: ScanPrimitiveType>(&self) -> Option<T> {
		match self {
			ScanEntryData::u8(v) => T::try_cast(*v),
			ScanEntryData::u16(v) => T::try_cast(*v),
			ScanEntryData::u32(v) => T::try_cast(*v),
			ScanEntryData::u64(v) => T::try_cast(*v),
			ScanEntryData::usize(v) => T::try_cast(*v),
			ScanEntryData::f32(v) => T::try_cast(*v),
			ScanEntryData::f64(v) => T::try_cast(*v),
		}
	}
}

#[derive(Debug, PartialEq)]
pub struct ScanEntry {
	/// Offset in the memory - basically the pointer into the process memory space.
	pub offset: usize,
	/// Data at the scanned offset.
	pub data: ScanEntryData
}
impl ScanEntry {
	pub fn u8(offset: usize, data: u8) -> Self {
		ScanEntry {
			offset,
			data: ScanEntryData::u8(data)
		}
	}

	pub fn u16(offset: usize, data: u16) -> Self {
		ScanEntry {
			offset,
			data: ScanEntryData::u16(data)
		}
	}

	pub fn u32(offset: usize, data: u32) -> Self {
		ScanEntry {
			offset,
			data: ScanEntryData::u32(data)
		}
	}

	pub fn u64(offset: usize, data: u64) -> Self {
		ScanEntry {
			offset,
			data: ScanEntryData::u64(data)
		}
	}

	pub fn usize(offset: usize, data: usize) -> Self {
		ScanEntry {
			offset,
			data: ScanEntryData::usize(data)
		}
	}

	pub fn f32(offset: usize, data: f32) -> Self {
		ScanEntry {
			offset,
			data: ScanEntryData::f32(data)
		}
	}

	pub fn f64(offset: usize, data: f64) -> Self {
		ScanEntry {
			offset,
			data: ScanEntryData::f64(data)
		}
	}
}
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ScanFlow {
	Continue,
	Break
}
