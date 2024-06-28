use std::cmp::Ordering;

use crate::{
    battle::BattleEngineSpeedSortTieResolution,
    rng::{
        rand_util,
        PseudoRandomNumberGenerator,
    },
};

/// An object that can be ordered by speed.
pub trait SpeedOrderable {
    fn order(&self) -> u32;
    fn priority(&self) -> i32;
    fn speed(&self) -> u32;
    fn sub_order(&self) -> u32;
}

impl<T> SpeedOrderable for &'_ T
where
    T: SpeedOrderable,
{
    #[inline]
    fn order(&self) -> u32 {
        (*self).order()
    }
    #[inline]
    fn priority(&self) -> i32 {
        (*self).priority()
    }
    #[inline]
    fn speed(&self) -> u32 {
        (*self).speed()
    }
    #[inline]
    fn sub_order(&self) -> u32 {
        (*self).sub_order()
    }
}

/// Compares the priority of two objects.
pub fn compare_priority<'a, T>(a: &'a T, b: &'a T) -> Ordering
where
    &'a T: SpeedOrderable,
{
    // Lower order first.
    a.order().cmp(&b.order()).then_with(|| {
        // Higher priority first.
        b.priority().cmp(&a.priority()).then_with(|| {
            // Higher speed first.
            b.speed()
                .cmp(&a.speed())
                // Lower sub order first.
                .then_with(|| a.sub_order().cmp(&b.sub_order()))
        })
    })
}

// Selection sort implementation that shuffles tied elements.
fn sort_with_random_ties<T, C>(
    items: &mut [T],
    comp: C,
    prng: &mut dyn PseudoRandomNumberGenerator,
    tie_resolution: BattleEngineSpeedSortTieResolution,
) where
    C: Fn(&T, &T) -> Ordering,
{
    let mut shuffler = |items: &mut [T]| rand_util::shuffle(prng, items);
    let mut sorted = 0;
    while sorted + 1 < items.len() {
        // Find all indices that are tied for the smallest elements.
        let mut smallest_indices = Vec::from([sorted]);
        for i in (sorted + 1)..items.len() {
            match comp(&items[smallest_indices[0]], &items[i]) {
                Ordering::Greater => continue,
                Ordering::Less => smallest_indices = Vec::from([i]),
                Ordering::Equal => smallest_indices.push(i),
            }
        }
        // Move smallest elements to the beginning of the list.
        let ties = smallest_indices.len();
        for (i, item_index) in smallest_indices.into_iter().enumerate() {
            if item_index != sorted + i {
                items.swap(sorted + i, item_index);
            }
        }
        // Shuffle ties.
        if ties > 1 {
            match tie_resolution {
                BattleEngineSpeedSortTieResolution::Random => {
                    shuffler(&mut items[sorted..(sorted + ties)])
                }
                BattleEngineSpeedSortTieResolution::Keep => (),
                BattleEngineSpeedSortTieResolution::Reverse => {
                    items[sorted..(sorted + ties)].reverse()
                }
            }
        }
        sorted += ties;
        //items[0..sorted] is now sorted.
    }
}

/// Sorts the given items by speed.
pub fn speed_sort<T>(
    items: &mut [T],
    prng: &mut dyn PseudoRandomNumberGenerator,
    tie_resolution: BattleEngineSpeedSortTieResolution,
) where
    for<'a> &'a T: SpeedOrderable,
{
    sort_with_random_ties(items, |a, b| compare_priority(b, a), prng, tie_resolution);
}
