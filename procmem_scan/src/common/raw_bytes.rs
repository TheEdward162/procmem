/// Trait for types that can safely be represented and read as raw bytes.
///
/// Most notably it is UB to read padding bytes, so this trait cannot just be
/// implemented for any type.
///
/// ## Safety
/// * The type memory representation must be safe to read in its entirety - i.e. no padding bytes
/// * The types size must be a multiple of its alignment - so that `[T]` can also implement `AsRawBytes`
pub unsafe trait AsRawBytes {
	/// Returns a view of the memory covered by `self` as raw bytes.
	fn as_raw_bytes(&self) -> &[u8];

	/// Returns the alignment requirement of the type.
	///
	/// This is needed for when the implementor of this trait is a reference.
	/// Then this function returns the alignment of the type behind reference, not of the reference itself.
	fn align_of() -> usize;
}
macro_rules! impl_as_raw_bytes {
	(
		$raw_type: ty
	) => {
		unsafe impl AsRawBytes for $raw_type {
			fn as_raw_bytes(&self) -> &[u8] {
				unsafe {
					std::slice::from_raw_parts(
						self as *const $raw_type as *const u8,
						std::mem::size_of::<$raw_type>()
					)
				}
			}

			fn align_of() -> usize {
				std::mem::align_of::<$raw_type>()
			}
		}
	};

	(
		Array:
		$($num: literal),+ $(,)?
	) => {
		$(
			unsafe impl<T: AsRawBytes> AsRawBytes for [T; $num] {
				fn as_raw_bytes(&self) -> &[u8] {
					self.as_ref().as_raw_bytes()
				}

				fn align_of() -> usize {
					std::mem::align_of::<T>()
				}
			}
		)+
	};

	(
		Derefable:
		$(
			{ $($toks: tt)+ }
		),+ $(,)?
	) => {
		$(
			unsafe impl<T: AsRawBytes> AsRawBytes for $($toks)+ {
				fn as_raw_bytes(&self) -> &[u8] {
					(**self).as_raw_bytes()
				}
			
				fn align_of() -> usize {
					std::mem::align_of::<T>()
				}
			}
		)+
	};
	
	(
		$(
			$raw_type: ty
		),+ $(,)?
	) => {
		$(
			impl_as_raw_bytes!($raw_type);
		)+
	};
}
impl_as_raw_bytes!(
	u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, usize, isize,
	f32, f64
);
unsafe impl<T: AsRawBytes> AsRawBytes for &T {
	fn as_raw_bytes(&self) -> &[u8] {
		(*self).as_raw_bytes()
	}

	fn align_of() -> usize {
		std::mem::align_of::<T>()
	}
}
unsafe impl<T: AsRawBytes> AsRawBytes for [T] {
	fn as_raw_bytes(&self) -> &[u8] {
		// This is safe because `T` must implement `AsRawBytes`
		// and thus must be safe for reinterpreting as raw bytes.
		unsafe {
			std::slice::from_raw_parts(
				self.as_ptr() as *const u8,
				std::mem::size_of::<T>() * self.len()
			)
		}
	}

	fn align_of() -> usize {
		std::mem::align_of::<T>()
	}
}
impl_as_raw_bytes!(
	Derefable:
	{ Vec<T> },
	{ Box<T> },
	{ Box<[T]> },
	{ std::rc::Rc<T> },
	{ std::rc::Rc<[T]> },
	{ std::sync::Arc<T> },
	{ std::sync::Arc<[T]> },
);

unsafe impl AsRawBytes for str {
	fn as_raw_bytes(&self) -> &[u8] {
		self.as_bytes().as_raw_bytes()
	}

	fn align_of() -> usize {
		std::mem::align_of::<u8>()
	}
}
unsafe impl AsRawBytes for String {
	fn as_raw_bytes(&self) -> &[u8] {
		self.as_bytes().as_raw_bytes()
	}

	fn align_of() -> usize {
		std::mem::align_of::<u8>()
	}
}
impl_as_raw_bytes!(
	Array:
	1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16
);

#[cfg(test)]
mod test {
	use super::AsRawBytes;

	#[test]
	fn test_as_raw_bytes() {
		let v = 1u16;

		assert_eq!(
			v.as_raw_bytes(),
			v.to_ne_bytes()
		);
	}

	#[test]
	fn test_as_raw_bytes_vec() {
		let v = vec![std::u16::MAX; 2];

		assert_eq!(
			v.as_raw_bytes(),
			&[0xFF, 0xFF, 0xFF, 0xFF]
		);
	}

	#[test]
	fn test_as_raw_bytes_array() {
		let v = [std::i32::MAX, std::i32::MIN];
		let e = {
			let f = std::i32::MAX.to_ne_bytes();
			let s = std::i32::MIN.to_ne_bytes();

			[f[0], f[1], f[2], f[3], s[0], s[1], s[2], s[3]]
		};

		assert_eq!(
			v.as_raw_bytes(),
			&e
		);
	}
}