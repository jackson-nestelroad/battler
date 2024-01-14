use rand::Rng;

/// A pseudo-random number generator, created with the intention of using a random number generator
/// that can be deterministically "replayed" for battle simulations.
pub trait PseudoRandomNumberGenerator {
    /// Returns the initial seed the generator was created with.
    ///
    /// The initial seed can be used to replay the random number generation sequence.
    fn initial_seed(&self) -> u64;

    /// Returns the next integer in the sequence.
    fn next(&mut self) -> u64;
}

pub struct RealPseudoRandomNumberGenerator {
    initial_seed: u64,
    seed: u64,
}

impl RealPseudoRandomNumberGenerator {
    /// Creates a new random number generator.
    pub fn new() -> Self {
        Self::new_with_seed(Self::generate_seed())
    }

    /// Creates a new random number generator with the given seed.
    ///
    /// If two random number generators are created with the same seed, their output should be
    /// exactly the same.
    pub fn new_with_seed(seed: u64) -> Self {
        Self {
            initial_seed: seed,
            seed,
        }
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
}

impl PseudoRandomNumberGenerator for RealPseudoRandomNumberGenerator {
    fn initial_seed(&self) -> u64 {
        self.initial_seed
    }

    fn next(&mut self) -> u64 {
        self.seed = Self::next_seed(self.seed);
        // Use the upper 32 bits. The lower ones are predictable in some situations.
        self.seed >> 32
    }
}

#[cfg(test)]
mod prng_tests {
    use crate::rng::{
        PseudoRandomNumberGenerator,
        RealPseudoRandomNumberGenerator,
    };

    #[test]
    fn stores_initial_seed() {
        assert_eq!(
            RealPseudoRandomNumberGenerator::new_with_seed(12345).initial_seed(),
            12345
        );
        assert_eq!(
            RealPseudoRandomNumberGenerator::new_with_seed(6789100000).initial_seed(),
            6789100000
        );
    }
}
