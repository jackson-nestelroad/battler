pub mod rand_util;

use std::any::Any;

use rand::Rng;

/// A pseudo-random number generator, created with the intention of using a random number generator
/// that can be deterministically "replayed" for battle simulations.
pub trait PseudoRandomNumberGenerator: Send + Sync {
    /// Returns the initial seed the generator was created with.
    ///
    /// The initial seed can be used to replay the random number generation sequence.
    fn initial_seed(&self) -> u64;

    /// Returns the next integer in the sequence.
    fn next(&mut self) -> u64;

    /// Mutable cast to [`Any`]` for testing.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// A real implementation of [`PseudoRandomNumberGenerator`].
pub struct RealPseudoRandomNumberGenerator {
    initial_seed: u64,
    seed: u64,
}

impl RealPseudoRandomNumberGenerator {
    /// Creates a new random number generator.
    ///
    /// If two random number generators are created with the same seed, their output should be
    /// exactly the same.
    pub fn new(seed: Option<u64>) -> Self {
        let seed = seed.unwrap_or_else(|| Self::generate_seed());
        Self {
            initial_seed: seed,
            seed,
        }
    }

    fn generate_seed() -> u64 {
        let mut rng = rand::rng();
        rng.random()
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

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod prng_test {
    use crate::{
        PseudoRandomNumberGenerator,
        RealPseudoRandomNumberGenerator,
    };

    #[test]
    fn stores_initial_seed() {
        assert_eq!(
            RealPseudoRandomNumberGenerator::new(Some(12345)).initial_seed(),
            12345
        );
        assert_eq!(
            RealPseudoRandomNumberGenerator::new(Some(6789100000)).initial_seed(),
            6789100000
        );
    }
}
