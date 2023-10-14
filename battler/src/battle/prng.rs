use rand::Rng;
use std::mem;

#[cfg(test)]
use std::collections::VecDeque;

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
        self.seed
    }

    /// Returns whether a random event occurs.
    pub fn chance(&mut self, numerator: u64, denominator: u64) -> bool {
        self.next().rem_euclid(denominator) < numerator
    }

    /// Returns a random integer in the range `[min, max)`.
    pub fn range(&mut self, min: u64, max: u64) -> u64 {
        self.next().rem_euclid(max - min) + min
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
            true, true, false, false, false, false, true, true, false, false, false, false, true,
            false, true, false, true, false, true, false, true, false, false, false, false, true,
            true, true, true, true, true, false, true, false, false,
        ];
        let got = (0..35).map(|_| prng.chance(num, den)).collect::<Vec<_>>();
        assert_eq!(got, want);
    }

    #[test]
    fn shuffles_slice() {
        let mut prng = PseudoRandomNumberGenerator::new_with_seed(123456789);
        let mut items = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        prng.shuffle(&mut items);
        let mut want = [6, 3, 8, 4, 0, 2, 5, 1, 7, 9];
        assert_eq!(items, want);
        prng.shuffle(&mut items);
        want = [4, 7, 6, 2, 1, 0, 8, 5, 9, 3];
        assert_eq!(items, want);
        prng.shuffle(&mut items);
        want = [9, 7, 6, 1, 8, 4, 2, 3, 0, 5];
        assert_eq!(items, want);
    }
}
