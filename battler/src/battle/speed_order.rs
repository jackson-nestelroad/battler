use std::cmp::Ordering;

use battler_prng::{
    PseudoRandomNumberGenerator,
    rand_util,
};

use crate::battle::CoreBattleEngineSpeedSortTieResolution;

/// An object that can be ordered by speed.
pub trait SpeedOrderable {
    /// Order. Lowest order goes first.
    fn order(&self) -> u32;
    /// Priority. Highest priority goes first.
    fn priority(&self) -> i32;
    /// Sub-priority. Highest priority goes first.
    fn sub_priority(&self) -> i32;
    /// Speed. Highest speed goes first.
    fn speed(&self) -> u32;
    /// Sub-order. Lowest order goes first.
    fn sub_order(&self) -> u32 {
        0
    }
    /// Effect order. Lowest order goes first.
    fn effect_order(&self) -> u32 {
        0
    }
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
    fn sub_priority(&self) -> i32 {
        (*self).sub_priority()
    }
    #[inline]
    fn speed(&self) -> u32 {
        (*self).speed()
    }
    #[inline]
    fn sub_order(&self) -> u32 {
        (*self).sub_order()
    }
    #[inline]
    fn effect_order(&self) -> u32 {
        (*self).effect_order()
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
            // Higher sub-priority first.
            b.sub_priority().cmp(&a.sub_priority()).then_with(|| {
                // Higher speed first.
                b.speed()
                    .cmp(&a.speed())
                    // Lower sub-order first.
                    .then_with(|| {
                        a.sub_order()
                            .cmp(&b.sub_order())
                            .then_with(|| a.effect_order().cmp(&b.effect_order()))
                    })
            })
        })
    })
}

fn stable_move_to_position<T>(items: &mut [T], index: usize, target: usize) {
    if target == index {
        return;
    } else if index < target {
        for i in index..target {
            items.swap(i, i + 1);
        }
    } else {
        for i in ((target + 1)..=index).rev() {
            items.swap(i - 1, i);
        }
    }
}

// Selection sort implementation that shuffles tied elements.
pub fn sort_with_random_ties<T, C>(
    items: &mut [T],
    comp: C,
    prng: &mut dyn PseudoRandomNumberGenerator,
    tie_resolution: CoreBattleEngineSpeedSortTieResolution,
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
            // Stable sort to make testing much easier.
            stable_move_to_position(items, item_index, sorted + i);
        }
        // Shuffle ties.
        if ties > 1 {
            match tie_resolution {
                CoreBattleEngineSpeedSortTieResolution::Random => {
                    shuffler(&mut items[sorted..(sorted + ties)])
                }
                CoreBattleEngineSpeedSortTieResolution::Keep => (),
                CoreBattleEngineSpeedSortTieResolution::Reverse => {
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
    tie_resolution: CoreBattleEngineSpeedSortTieResolution,
) where
    for<'a> &'a T: SpeedOrderable,
{
    sort_with_random_ties(items, |a, b| compare_priority(b, a), prng, tie_resolution);
}
