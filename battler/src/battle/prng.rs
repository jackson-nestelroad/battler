#[cfg(test)]
use std::collections::VecDeque;
use std::mem;

use rand::Rng;

/// A pseudo-random number generator, created with the intention of using a random number generator
/// that can be deterministically "replayed" for battle simulations.
pub struct PseudoRandomNumberGenerator {
    initial_seed: u64,
    seed: u64,
    // Test-only field that allows individual values to be returned.
    #[cfg(test)]
    test_values: VecDeque<u64>,
}

impl PseudoRandomNumberGenerator {
    pub fn new() -> Self {
        Self::new_with_seed(Self::generate_seed())
    }

    pub fn new_with_seed(seed: u64) -> Self {
        Self {
            initial_seed: seed,
            seed,
            #[cfg(test)]
            test_values: VecDeque::new(),
        }
    }

    /// Returns the initial seed, which can be used to replay the random number generation sequence.
    pub fn initial_seed(&self) -> u64 {
        self.initial_seed
    }

    fn generate_seed() -> u64 {
        let mut rng = rand::thread_rng();
        rng.gen()
    }

    /// Linear Congruential Generator (LCRNG).
    fn next_seed(seed: u64) -> u64 {
        // Constants in the generation V and VI games.
        const A: u64 = 0x5D588B656C078965;
        const C: u64 = 0x0000000000269EC3;
        seed.wrapping_mul(A).overflowing_add(C).0
    }

    fn next(&mut self) -> u64 {
        #[cfg(test)]
        if let Some(next) = self.test_values.pop_front() {
            return next;
        }
        self.seed = Self::next_seed(self.seed);
        // Use the upper 32 bits. The lower ones are predictable in some situations.
        self.seed >> 32
    }

    /// Returns whether a random event occurs.
    pub fn chance(&mut self, numerator: u64, denominator: u64) -> bool {
        self.next().rem_euclid(denominator) < numerator
    }

    /// Returns a random integer in the range `[min, max)`.
    pub fn range(&mut self, min: u64, max: u64) -> u64 {
        self.next().rem_euclid(max - min) + min
    }

    /// Returns a random value from the given iterator.
    pub fn sample_iter<'a, I, T>(&mut self, iter: I) -> Result<T, &'static str>
    where
        I: Iterator<Item = T>,
        T: Clone,
    {
        let items = iter.collect::<Vec<_>>();
        if items.is_empty() {
            return Err("cannot sample an empty iterator");
        }
        let index = self.range(0, items.len() as u64);
        unsafe { Ok(items.get_unchecked(index as usize).clone()) }
    }

    /// Returns a random element from the given slice.
    pub fn sample_slice<'a, T>(&mut self, slice: &'a [T]) -> Result<&'a T, &'static str> {
        if slice.is_empty() {
            return Err("cannot sample an empty slice");
        }
        let index = self.range(0, slice.len() as u64);
        unsafe { Ok(slice.get_unchecked(index as usize)) }
    }

    /// Shuffles the given slice using a Fisher-Yates shuffle.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        let mut start = 0;
        let end = items.len() as u64;
        while start < end - 1 {
            let next = self.range(start, end);
            if start != next {
                let (head, tail) = items.split_at_mut(next as usize);
                mem::swap(&mut head[start as usize], &mut tail[0]);
            }
            start += 1;
        }
    }
}

#[cfg(test)]
mod prng_tests {
    use crate::battle::PseudoRandomNumberGenerator;

    #[test]
    fn stores_initial_seed() {
        assert_eq!(
            PseudoRandomNumberGenerator::new_with_seed(12345).initial_seed(),
            12345
        );
        assert_eq!(
            PseudoRandomNumberGenerator::new_with_seed(6789100000).initial_seed(),
            6789100000
        );
    }

    #[test]
    fn generates_number_in_range() {
        let mut prng = PseudoRandomNumberGenerator::new();
        let min = 5;
        let max = 12;
        for _ in 0..50 {
            let n = prng.range(min, max);
            assert!(n >= min);
            assert!(n < max);
        }
    }

    #[test]
    fn generates_chance() {
        let mut prng = PseudoRandomNumberGenerator::new_with_seed(100);
        let num = 3;
        let den = 7;
        let want = vec![
            true, true, false, false, false, false, false, true, true, true, false, true, true,
            false, true, false, true, false, false, true, true, true, true, true, false, false,
            true, false, false, false, true, false, false, false, false,
        ];
        let got = (0..35).map(|_| prng.chance(num, den)).collect::<Vec<_>>();
        assert_eq!(got, want);
    }

    #[test]
    fn shuffles_slice() {
        let mut prng = PseudoRandomNumberGenerator::new_with_seed(123456789);
        let mut items = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        prng.shuffle(&mut items);
        let mut want = [3, 4, 9, 6, 0, 1, 2, 5, 7, 8];
        assert_eq!(items, want);
        prng.shuffle(&mut items);
        want = [6, 7, 3, 4, 8, 2, 1, 0, 9, 5];
        assert_eq!(items, want);
        prng.shuffle(&mut items);
        want = [6, 8, 7, 4, 5, 2, 3, 9, 1, 0];
        assert_eq!(items, want);
    }

    #[test]
    fn sample_iter_fails_empty_iterator() {
        let mut prng = PseudoRandomNumberGenerator::new_with_seed(123456789);
        let items: Vec<&str> = Vec::new();
        assert_eq!(
            prng.sample_iter(items.iter()),
            Err("cannot sample an empty iterator")
        );
    }

    #[test]
    fn samples_element_in_iterator() {
        let mut prng = PseudoRandomNumberGenerator::new_with_seed(123456789);
        let items = vec!["a", "b", "c", "d"];
        assert_eq!(prng.sample_iter(items.iter()), Ok(&"d"));
        assert_eq!(prng.sample_iter(items.iter()), Ok(&"a"));
        assert_eq!(prng.sample_iter(items.iter()), Ok(&"d"));
        assert_eq!(prng.sample_iter(items.iter()), Ok(&"d"));
        assert_eq!(prng.sample_iter(items.iter()), Ok(&"c"));
        assert_eq!(prng.sample_iter(items.iter()), Ok(&"c"));
        assert_eq!(prng.sample_iter(items.iter()), Ok(&"d"));
        assert_eq!(prng.sample_iter(items.iter()), Ok(&"d"));
        assert_eq!(prng.sample_iter(items.iter()), Ok(&"b"));
        assert_eq!(prng.sample_iter(items.iter()), Ok(&"b"));
    }

    #[test]
    fn samples_element_in_slice() {
        let mut prng = PseudoRandomNumberGenerator::new_with_seed(987654321);
        let items = vec!["a", "b", "c", "d"];
        assert_eq!(prng.sample_slice(&items), Ok(&"a"));
        assert_eq!(prng.sample_slice(&items), Ok(&"b"));
        assert_eq!(prng.sample_slice(&items), Ok(&"a"));
        assert_eq!(prng.sample_slice(&items), Ok(&"a"));
        assert_eq!(prng.sample_slice(&items), Ok(&"a"));
        assert_eq!(prng.sample_slice(&items), Ok(&"b"));
        assert_eq!(prng.sample_slice(&items), Ok(&"d"));
        assert_eq!(prng.sample_slice(&items), Ok(&"c"));
        assert_eq!(prng.sample_slice(&items), Ok(&"c"));
        assert_eq!(prng.sample_slice(&items), Ok(&"d"));
    }

    #[test]
    fn sample_iter_fails_empty_slice() {
        let mut prng = PseudoRandomNumberGenerator::new_with_seed(987654321);
        let items: Vec<&str> = Vec::new();
        assert_eq!(
            prng.sample_slice(&items),
            Err("cannot sample an empty slice")
        );
    }
}
