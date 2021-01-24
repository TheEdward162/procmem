/// An iterator that is a hybrid of `filter` and `fold`.
///
/// Like `filter`, each iteration may or may not yield a value.
///
/// Like `fold`, there is an accumulator element, albeit optional.
/// Unlike `fold` however, it may return one item for each original item.
/// Additionally, the accumulated value is yielded in the end.
///
/// ## Example
/// ```
/// # use procmem::util::AccFilter;
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
pub struct AccFilter<I: Iterator<Item = T>, F: FnMut(&mut Option<T>, T) -> Option<T>, T> {
	iter: I,
	fun: F,
	state: Option<T>
}
impl<I: Iterator<Item = T>, F: FnMut(&mut Option<T>, T) -> Option<T>, T> AccFilter<I, F, T> {
	pub fn new(iter: I, fun: F) -> Self {
		AccFilter {
			iter,
			fun,
			state: None
		}
	}
}
impl<I: Iterator<Item = T>, F: FnMut(&mut Option<T>, T) -> Option<T>, T> Iterator
	for AccFilter<I, F, T>
{
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.iter.next() {
				None => break self.state.take(),
				Some(item) => match (self.fun)(&mut self.state, item) {
					None => continue,
					Some(result) => break Some(result)
				}
			}
		}
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
					_ => acc.replace(curr)
				}
			);

		let deduped = dedup.collect::<Vec<_>>();
		assert_eq!(deduped, &[1, 2, 3, 4]);
	}
}
