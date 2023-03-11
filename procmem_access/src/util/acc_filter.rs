/// An iterator that is a hybrid of `filter` and `fold_first`.
///
/// Like `fold_first`, there is an accumulator element. Unlike `fold` however,
/// the accumulator element is not prepopulated with the first item of the inner
/// iterator and is optional.
///
/// Like `filter`, each iteration may or may not yield a value.
///
/// This iterator may yield between 0 and N + 1 items (where N is the number of items yielded by the inner iterator).
///
/// ## Example
/// ```
/// # use procmem_access::util::AccFilter;
/// let dedup = AccFilter::new(
/// 	[1, 1, 1, 2, 3, 3, 4, 4, 4].iter().copied(),
/// 	|acc, curr| match acc {
/// 		Some(acc) if *acc == curr => None,
/// 		_ => acc.replace(curr)
/// 	}
/// );
///
/// let deduped = dedup.collect::<Vec<_>>();
/// assert_eq!(
/// 	deduped,
/// 	&[1, 2, 3, 4]
/// );
/// ```
pub struct AccFilter<T, I: Iterator<Item = T>, F: FnMut(&mut Option<T>, T) -> Option<T>> {
	iter: I,
	fun: F,
	state: Option<T>,
}
impl<T, I: Iterator<Item = T>, F: FnMut(&mut Option<T>, T) -> Option<T>> AccFilter<T, I, F> {
	pub fn new(iter: I, fun: F) -> Self {
		AccFilter {
			iter,
			fun,
			state: None,
		}
	}
}
impl<T, F: FnMut(&mut Option<T>, T) -> Option<T>> AccFilter<T, std::iter::Empty<T>, F> {
	/// Performs accumulation filter on a vector in-place.
	pub fn acc_filter_vec_mut(vec: &mut Vec<T>, mut fun: F) {
		// reserve one more because we might produce one more values than there are originally
		vec.reserve(1);
		let vec_ptr = vec.as_mut_ptr();
		let vec_len = vec.len();

		// ensure panic safety
		// we are going to manually move around values backed by this memory
		// and cannot let a panic in `fun` cause a double-drop for non-copy Ts
		unsafe {
			vec.set_len(0);
		}

		let mut acc = None;
		let mut write_index = 0;
		for read_index in 0..vec_len {
			// move a value out of the vector
			// safe because the vec already fulfills the requirements
			// and because we `set_len(0)` panics don't cause a double-drop
			let value = unsafe { std::ptr::read(vec_ptr.add(read_index)) };

			match fun(&mut acc, value) {
				None => (),
				Some(value) => {
					// move the produced value into the vector
					// safe because the closure can never produce more elements than it receives
					// (plus the one in acc handled later)
					unsafe {
						std::ptr::write(vec_ptr.add(write_index), value);
					}
					write_index += 1;
				}
			}
		}

		if let Some(acc) = acc {
			// safe because we reserved the length + 1
			unsafe {
				std::ptr::write(vec_ptr.add(write_index), acc);
			}
			write_index += 1;
		}

		// restore vec len to how may elements were preserved
		// safe because write_index is at most `vec_len + 1`
		unsafe {
			vec.set_len(write_index);
		}
	}
}
impl<T, I: Iterator<Item = T>, F: FnMut(&mut Option<T>, T) -> Option<T>> Iterator
	for AccFilter<T, I, F>
{
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.iter.next() {
				None => break self.state.take(),
				Some(item) => match (self.fun)(&mut self.state, item) {
					None => continue,
					Some(result) => break Some(result),
				},
			}
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let upper = match self.iter.size_hint().1 {
			None => None,
			Some(u) => u.checked_add(1),
		};

		(0, upper)
	}
}

#[cfg(test)]
mod test {
	use super::AccFilter;

	#[test]
	fn test_acc_filter() {
		let dedup =
			AccFilter::new(
				[1, 1, 1, 2, 3, 3, 4, 4, 4].iter().copied(),
				|acc, curr| match acc {
					Some(acc) if *acc == curr => None,
					_ => acc.replace(curr),
				},
			);

		let deduped = dedup.collect::<Vec<_>>();
		assert_eq!(deduped, &[1, 2, 3, 4]);
	}

	#[test]
	fn test_acc_filter_vec_mut() {
		let mut vec = vec![1, 1, 1, 2, 3, 3, 4, 4, 4];

		AccFilter::acc_filter_vec_mut(&mut vec, |acc, curr| match acc {
			Some(acc) if *acc == curr => None,
			_ => acc.replace(curr),
		});

		assert_eq!(vec, &[1, 2, 3, 4]);
	}
}
