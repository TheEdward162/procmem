use super::ScanPrimitiveType;

#[derive(Debug)]
#[repr(C, align(8))]
pub struct ByteScanner {
	buffer: [u8; 8],
	count: usize
}
impl ByteScanner {
	pub const BUFFER_SIZE: usize = 8;

	pub fn new() -> Self {
		ByteScanner {
			buffer: Default::default(),
			count: 0
		}
	}

	pub fn push(&mut self, byte: u8) -> usize {
		for i in 0 .. Self::BUFFER_SIZE - 1 {
			self.buffer[i] = self.buffer[i + 1];
		}
		self.buffer[Self::BUFFER_SIZE - 1] = byte;

		self.count += 1;
		self.count
	}

	/// Returns true if enough bytes from an aligned `T` have been pushed.
	pub fn ready<T: ScanPrimitiveType>(&self) -> bool {
		debug_assert_ne!(std::mem::size_of::<T>(), 0);

		self.count % std::mem::size_of::<T>() == 0 && self.count != 0
	}

	/// Returns `true` if enough bytes from an unaligned `T` have been pushed.
	///
	/// After this method returns `true` once, it will keep returning `true` until the next reset.
	///
	/// Unaligned values are values that would not be aligned correctly
	/// assuming the first byte pushed is correctly aligned.
	pub fn ready_unaligned<T: ScanPrimitiveType>(&self) -> bool {
		debug_assert_ne!(std::mem::size_of::<T>(), 0);

		self.count >= std::mem::size_of::<T>()
	}

	/// Returns number of bytes pushed.
	pub fn count(&self) -> usize {
		self.count
	}

	/// Resets the internal counter, treating the next push as if it was the first.
	pub fn reset(&mut self) {
		self.count = 0;
	}

	/// While not a requirement, `self.ready_unaligned<T>` should be true before calling this.
	///
	/// This function is not unsafe because it relies on `ScanPrimitiveType` to provide types which
	/// are have full range of valid memory representations.
	pub fn read<T: ScanPrimitiveType>(&self) -> T {
		debug_assert!(std::mem::size_of::<T>() <= Self::BUFFER_SIZE);

		// This is safe because ScanPrimitiveType trait guarantees valid memory representation
		unsafe {
			let ptr = self
				.buffer
				.as_ptr()
				.add(Self::BUFFER_SIZE - std::mem::size_of::<T>()) as *const T;

			*ptr
		}
	}
}

#[cfg(test)]
mod test {
	use super::ByteScanner;

	#[test]
	fn test_byte_scanner() {
		let mut scanner = ByteScanner::new();

		{
			assert_eq!(scanner.ready::<u64>(), false);
			assert_eq!(scanner.ready_unaligned::<u64>(), false);
			assert_eq!(scanner.ready::<u32>(), false);
			assert_eq!(scanner.ready_unaligned::<u32>(), false);
			assert_eq!(scanner.ready::<u16>(), false);
			assert_eq!(scanner.ready_unaligned::<u16>(), false);
			assert_eq!(scanner.ready::<u8>(), false);
			assert_eq!(scanner.ready_unaligned::<u8>(), false);
		}

		{
			assert_eq!(scanner.push(1), 1);

			assert_eq!(scanner.ready::<u64>(), false);
			assert_eq!(scanner.ready_unaligned::<u64>(), false);

			assert_eq!(scanner.ready::<u32>(), false);
			assert_eq!(scanner.ready_unaligned::<u32>(), false);

			assert_eq!(scanner.ready::<u16>(), false);
			assert_eq!(scanner.ready_unaligned::<u16>(), false);

			assert_eq!(scanner.ready::<u8>(), true);
			assert_eq!(scanner.ready_unaligned::<u8>(), true);
			assert_eq!(scanner.read::<u8>(), 1);
		}

		{
			assert_eq!(scanner.push(2), 2);

			assert_eq!(scanner.ready::<u64>(), false);
			assert_eq!(scanner.ready_unaligned::<u64>(), false);

			assert_eq!(scanner.ready::<u32>(), false);
			assert_eq!(scanner.ready_unaligned::<u32>(), false);

			assert_eq!(scanner.ready::<u16>(), true);
			assert_eq!(scanner.ready_unaligned::<u16>(), true);
			assert_eq!(scanner.read::<u16>(), 1 + (2 << 8));

			assert_eq!(scanner.ready::<u8>(), true);
			assert_eq!(scanner.ready_unaligned::<u8>(), true);
			assert_eq!(scanner.read::<u8>(), 2);
		}

		{
			assert_eq!(scanner.push(3), 3);

			assert_eq!(scanner.ready::<u64>(), false);
			assert_eq!(scanner.ready_unaligned::<u64>(), false);

			assert_eq!(scanner.ready::<u32>(), false);
			assert_eq!(scanner.ready_unaligned::<u32>(), false);

			assert_eq!(scanner.ready::<u16>(), false);
			assert_eq!(scanner.ready_unaligned::<u16>(), true);
			assert_eq!(scanner.read::<u16>(), 2 + (3 << 8));

			assert_eq!(scanner.ready::<u8>(), true);
			assert_eq!(scanner.ready_unaligned::<u8>(), true);
			assert_eq!(scanner.read::<u8>(), 3);
		}

		{
			assert_eq!(scanner.push(4), 4);

			assert_eq!(scanner.ready::<u64>(), false);
			assert_eq!(scanner.ready_unaligned::<u64>(), false);

			assert_eq!(scanner.ready::<u32>(), true);
			assert_eq!(scanner.ready_unaligned::<u32>(), true);
			assert_eq!(scanner.read::<u32>(), 1 + (2 << 8) + (3 << 16) + (4 << 24));

			assert_eq!(scanner.ready::<u16>(), true);
			assert_eq!(scanner.ready_unaligned::<u16>(), true);
			assert_eq!(scanner.read::<u16>(), 3 + (4 << 8));

			assert_eq!(scanner.ready::<u8>(), true);
			assert_eq!(scanner.ready_unaligned::<u8>(), true);
			assert_eq!(scanner.read::<u8>(), 4);
		}

		{
			assert_eq!(scanner.push(5), 5);

			assert_eq!(scanner.ready::<u64>(), false);
			assert_eq!(scanner.ready_unaligned::<u64>(), false);

			assert_eq!(scanner.ready::<u32>(), false);
			assert_eq!(scanner.ready_unaligned::<u32>(), true);
			assert_eq!(scanner.read::<u32>(), 2 + (3 << 8) + (4 << 16) + (5 << 24));

			assert_eq!(scanner.ready::<u16>(), false);
			assert_eq!(scanner.ready_unaligned::<u16>(), true);
			assert_eq!(scanner.read::<u16>(), 4 + (5 << 8));

			assert_eq!(scanner.ready::<u8>(), true);
			assert_eq!(scanner.ready_unaligned::<u8>(), true);
			assert_eq!(scanner.read::<u8>(), 5);
		}

		{
			assert_eq!(scanner.push(6), 6);

			assert_eq!(scanner.ready::<u64>(), false);
			assert_eq!(scanner.ready_unaligned::<u64>(), false);

			assert_eq!(scanner.ready::<u32>(), false);
			assert_eq!(scanner.ready_unaligned::<u32>(), true);
			assert_eq!(scanner.read::<u32>(), 3 + (4 << 8) + (5 << 16) + (6 << 24));

			assert_eq!(scanner.ready::<u16>(), true);
			assert_eq!(scanner.ready_unaligned::<u16>(), true);
			assert_eq!(scanner.read::<u16>(), 5 + (6 << 8));

			assert_eq!(scanner.ready::<u8>(), true);
			assert_eq!(scanner.ready_unaligned::<u8>(), true);
			assert_eq!(scanner.read::<u8>(), 6);
		}

		{
			assert_eq!(scanner.push(7), 7);

			assert_eq!(scanner.ready::<u64>(), false);
			assert_eq!(scanner.ready_unaligned::<u64>(), false);

			assert_eq!(scanner.ready::<u32>(), false);
			assert_eq!(scanner.ready_unaligned::<u32>(), true);
			assert_eq!(scanner.read::<u32>(), 4 + (5 << 8) + (6 << 16) + (7 << 24));

			assert_eq!(scanner.ready::<u16>(), false);
			assert_eq!(scanner.ready_unaligned::<u16>(), true);
			assert_eq!(scanner.read::<u16>(), 6 + (7 << 8));

			assert_eq!(scanner.ready::<u8>(), true);
			assert_eq!(scanner.ready_unaligned::<u8>(), true);
			assert_eq!(scanner.read::<u8>(), 7);
		}

		{
			assert_eq!(scanner.push(8), 8);

			assert_eq!(scanner.ready::<u64>(), true);
			assert_eq!(scanner.ready_unaligned::<u64>(), true);
			assert_eq!(
				scanner.read::<u64>(),
				1 + (2 << 8)
					+ (3 << 16) + (4 << 24)
					+ (5 << 32) + (6 << 40)
					+ (7 << 48) + (8 << 56)
			);

			assert_eq!(scanner.ready::<u32>(), true);
			assert_eq!(scanner.ready_unaligned::<u32>(), true);
			assert_eq!(scanner.read::<u32>(), 5 + (6 << 8) + (7 << 16) + (8 << 24));

			assert_eq!(scanner.ready::<u16>(), true);
			assert_eq!(scanner.ready_unaligned::<u16>(), true);
			assert_eq!(scanner.read::<u16>(), 7 + (8 << 8));

			assert_eq!(scanner.ready::<u8>(), true);
			assert_eq!(scanner.ready_unaligned::<u8>(), true);
			assert_eq!(scanner.read::<u8>(), 8);
		}
	}
}
