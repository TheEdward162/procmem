use std::iter::Peekable;

/// Merge-sort like merge iterator.
pub struct MergeIter<T: PartialOrd, A: Iterator<Item = T>, B: Iterator<Item = T>> {
	a: Peekable<A>,
	b: Peekable<B>
}
impl<T: PartialOrd, A: Iterator<Item = T>, B: Iterator<Item = T>> MergeIter<T, A, B> {
	/// Creates a new merge iterator.
	///
	/// This will only function correctly both `a` and `b` are sorted.
	pub fn new(a: A, b: B) -> Self {
		MergeIter {
			a: a.peekable(),
			b: b.peekable()
		}
	}
}
impl<T: PartialOrd, A: Iterator<Item = T>, B: Iterator<Item = T>> Iterator for MergeIter<T, A, B> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		match (self.a.peek(), self.b.peek()) {
			(None, None) => None,
			(_, None) => self.a.next(),
			(None, _) => self.b.next(),
			(Some(left), Some(right)) => {
				if left
					.partial_cmp(right)
					.map(|o| o != std::cmp::Ordering::Greater)
					.unwrap_or(false)
				{
					self.a.next()
				} else {
					self.b.next()
				}
			}
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let a_hint = self.a.size_hint();
		let b_hint = self.b.size_hint();

		(
			a_hint.0 + b_hint.0,
			a_hint.1.and_then(
				|a| b_hint.1.and_then(|b| a.checked_add(b))
			)
		)
	}
}

#[cfg(test)]
mod test {
	use super::MergeIter;

	#[test]
	fn test_merge_iter() {
		let seq_a = [1, 2, 3, 4, 5, 17, 18, 19, 20];
		let seq_b = [4, 5, 6, 7, 11, 31];

		let mut iter = MergeIter::new(seq_a.iter(), seq_b.iter());

		assert_eq!(iter.next(), Some(&1));
		assert_eq!(iter.next(), Some(&2));
		assert_eq!(iter.next(), Some(&3));
		assert_eq!(iter.next(), Some(&4));
		assert_eq!(iter.next(), Some(&4));
		assert_eq!(iter.next(), Some(&5));
		assert_eq!(iter.next(), Some(&5));
		assert_eq!(iter.next(), Some(&6));
		assert_eq!(iter.next(), Some(&7));
		assert_eq!(iter.next(), Some(&11));
		assert_eq!(iter.next(), Some(&17));
		assert_eq!(iter.next(), Some(&18));
		assert_eq!(iter.next(), Some(&19));
		assert_eq!(iter.next(), Some(&20));
		assert_eq!(iter.next(), Some(&31));
	}
}
