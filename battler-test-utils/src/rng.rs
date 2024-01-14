use std::{
    any::Any,
    collections::hash_map::Entry,
    mem,
};

use ahash::{
    HashMap,
    HashMapExt,
};
use battler::{
    battle::PublicCoreBattle,
    rng::{
        PseudoRandomNumberGenerator,
        RealPseudoRandomNumberGenerator,
    },
};

/// A controlled random number generator, for tests that need fine-grained control over battle RNG.
pub struct ControlledRandomNumberGenerator {
    count: usize,
    fake_values: HashMap<usize, u64>,
    real: RealPseudoRandomNumberGenerator,
}

impl ControlledRandomNumberGenerator {
    pub fn new(seed: Option<u64>) -> Self {
        Self {
            count: 0,
            fake_values: HashMap::new(),
            real: RealPseudoRandomNumberGenerator::new(seed),
        }
    }
}

impl PseudoRandomNumberGenerator for ControlledRandomNumberGenerator {
    fn initial_seed(&self) -> u64 {
        self.real.initial_seed()
    }

    fn next(&mut self) -> u64 {
        // Roll the underlying RNG to keep the sequence consistent, even if we do not use the value.
        let next = self.real.next();
        self.count += 1;
        let fake_entry = self.fake_values.entry(self.count);
        match fake_entry {
            Entry::Occupied(fake_entry) => fake_entry.remove(),
            Entry::Vacant(_) => next,
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ControlledRandomNumberGenerator {
    pub fn sequence_count(&self) -> usize {
        self.count
    }

    pub fn insert_fake_value(&mut self, count: usize, value: u64) {
        self.fake_values.insert(count, value);
    }

    pub fn insert_fake_values<I>(&mut self, iterable: I)
    where
        I: IntoIterator<Item = (usize, u64)>,
    {
        self.fake_values.extend(iterable.into_iter());
    }

    pub fn insert_fake_values_relative_to_sequence_count<I>(&mut self, iterable: I)
    where
        I: IntoIterator<Item = (usize, u64)>,
    {
        self.fake_values.extend(
            iterable
                .into_iter()
                .map(|(count, value)| (count + self.count, value)),
        );
    }
}
pub fn get_controlled_rng_for_battle<'b>(
    battle: &'b mut PublicCoreBattle,
) -> Option<&'b mut ControlledRandomNumberGenerator> {
    (battle.internal.prng.as_mut().as_any_mut() as &mut dyn Any)
        .downcast_mut::<ControlledRandomNumberGenerator>()
}
