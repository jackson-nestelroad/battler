#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::mem;

use crate::PseudoRandomNumberGenerator;

/// Returns whether a random event occurs.
pub fn chance(
    prng: &mut dyn PseudoRandomNumberGenerator,
    numerator: u64,
    denominator: u64,
) -> bool {
    prng.next().rem_euclid(denominator) < numerator
}

/// Returns a random integer in the range `[min, max)`.
pub fn range(prng: &mut dyn PseudoRandomNumberGenerator, min: u64, max: u64) -> u64 {
    prng.next().rem_euclid(max - min) + min
}

/// Returns a random value from the given iterator.
#[cfg(feature = "alloc")]
pub fn sample_iter<'a, I, T>(prng: &mut dyn PseudoRandomNumberGenerator, iter: I) -> Option<T>
where
    I: Iterator<Item = T>,
    T: Clone,
{
    let items = iter.collect::<Vec<_>>();
    if items.is_empty() {
        return None;
    }
    let index = range(prng, 0, items.len() as u64);
    unsafe { Some(items.get_unchecked(index as usize).clone()) }
}

/// Returns a random element from the given slice.
pub fn sample_slice<'a, T>(
    prng: &mut dyn PseudoRandomNumberGenerator,
    slice: &'a [T],
) -> Option<&'a T> {
    if slice.is_empty() {
        return None;
    }
    if slice.len() == 1 {
        return slice.first();
    }
    let index = range(prng, 0, slice.len() as u64);
    unsafe { Some(slice.get_unchecked(index as usize)) }
}

/// Fisher-Yates shuffle.
pub fn shuffle<T>(prng: &mut dyn PseudoRandomNumberGenerator, items: &mut [T]) {
    let mut start = 0;
    let end = items.len() as u64;
    while start < end - 1 {
        let next = range(prng, start, end);
        if start != next {
            let (head, tail) = items.split_at_mut(next as usize);
            mem::swap(&mut head[start as usize], &mut tail[0]);
        }
        start += 1;
    }
}

#[cfg(test)]
mod rand_util_test {
    use alloc::{
        vec,
        vec::Vec,
    };

    use crate::{
        RealPseudoRandomNumberGenerator,
        rand_util,
    };

    #[test]
    fn generates_number_in_range() {
        let mut prng = RealPseudoRandomNumberGenerator::new(None);
        let min = 5;
        let max = 12;
        for _ in 0..50 {
            let n = rand_util::range(&mut prng, min, max);
            assert!(n >= min);
            assert!(n < max);
        }
    }

    #[test]
    fn generates_chance() {
        let mut prng = RealPseudoRandomNumberGenerator::new(Some(100));
        let num = 3;
        let den = 7;
        let want = vec![
            true, true, false, false, false, false, false, true, true, true, false, true, true,
            false, true, false, true, false, false, true, true, true, true, true, false, false,
            true, false, false, false, true, false, false, false, false,
        ];
        let got = (0..35)
            .map(|_| rand_util::chance(&mut prng, num, den))
            .collect::<Vec<_>>();
        assert_eq!(got, want);
    }

    #[test]
    fn shuffles_slice() {
        let mut prng = RealPseudoRandomNumberGenerator::new(Some(123456789));
        let mut items = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        rand_util::shuffle(&mut prng, &mut items);
        let mut want = [3, 4, 9, 6, 0, 1, 2, 5, 7, 8];
        assert_eq!(items, want);
        rand_util::shuffle(&mut prng, &mut items);
        want = [6, 7, 3, 4, 8, 2, 1, 0, 9, 5];
        assert_eq!(items, want);
        rand_util::shuffle(&mut prng, &mut items);
        want = [6, 8, 7, 4, 5, 2, 3, 9, 1, 0];
        assert_eq!(items, want);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn sample_iter_fails_empty_iterator() {
        let mut prng = RealPseudoRandomNumberGenerator::new(Some(123456789));
        let items: Vec<&str> = Vec::new();
        assert_eq!(rand_util::sample_iter(&mut prng, items.iter()), None);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn samples_element_in_iterator() {
        let mut prng = RealPseudoRandomNumberGenerator::new(Some(123456789));
        let items = vec!["a", "b", "c", "d"];
        assert_eq!(rand_util::sample_iter(&mut prng, items.iter()), Some(&"d"));
        assert_eq!(rand_util::sample_iter(&mut prng, items.iter()), Some(&"a"));
        assert_eq!(rand_util::sample_iter(&mut prng, items.iter()), Some(&"d"));
        assert_eq!(rand_util::sample_iter(&mut prng, items.iter()), Some(&"d"));
        assert_eq!(rand_util::sample_iter(&mut prng, items.iter()), Some(&"c"));
        assert_eq!(rand_util::sample_iter(&mut prng, items.iter()), Some(&"c"));
        assert_eq!(rand_util::sample_iter(&mut prng, items.iter()), Some(&"d"));
        assert_eq!(rand_util::sample_iter(&mut prng, items.iter()), Some(&"d"));
        assert_eq!(rand_util::sample_iter(&mut prng, items.iter()), Some(&"b"));
        assert_eq!(rand_util::sample_iter(&mut prng, items.iter()), Some(&"b"));
    }

    #[test]
    fn samples_element_in_slice() {
        let mut prng = RealPseudoRandomNumberGenerator::new(Some(987654321));
        let items = vec!["a", "b", "c", "d"];
        assert_eq!(rand_util::sample_slice(&mut prng, &items), Some(&"a"));
        assert_eq!(rand_util::sample_slice(&mut prng, &items), Some(&"b"));
        assert_eq!(rand_util::sample_slice(&mut prng, &items), Some(&"a"));
        assert_eq!(rand_util::sample_slice(&mut prng, &items), Some(&"a"));
        assert_eq!(rand_util::sample_slice(&mut prng, &items), Some(&"a"));
        assert_eq!(rand_util::sample_slice(&mut prng, &items), Some(&"b"));
        assert_eq!(rand_util::sample_slice(&mut prng, &items), Some(&"d"));
        assert_eq!(rand_util::sample_slice(&mut prng, &items), Some(&"c"));
        assert_eq!(rand_util::sample_slice(&mut prng, &items), Some(&"c"));
        assert_eq!(rand_util::sample_slice(&mut prng, &items), Some(&"d"));
    }

    #[test]
    fn sample_slice_fails_empty_slice() {
        let mut prng = RealPseudoRandomNumberGenerator::new(Some(987654321));
        let items: Vec<&str> = Vec::new();
        assert_eq!(rand_util::sample_slice(&mut prng, &items), None);
    }
}
