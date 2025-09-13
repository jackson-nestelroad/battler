use std::{
    borrow::Borrow,
    collections::BTreeSet,
};

use serde::{
    Deserialize,
    Serialize,
};

/// A value that requires discovery.
///
/// If a value is "known", a single value will be recorded. Otherwise, a set of "possible values"
/// will be stored. Values can be "recorded" (in which the new value takes precedence) or "merged"
/// (in which the two values take equal precedence).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscoveryRequired<T>
where
    T: Ord,
{
    /// A known value.
    #[serde(rename = "known")]
    Known(T),
    /// A set of possible values.
    #[serde(rename = "possibly_one_of")]
    PossibleValues(BTreeSet<T>),
}

impl<T> DiscoveryRequired<T>
where
    T: Ord,
{
    /// Checks if there are no values stored.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::PossibleValues(values) => values.is_empty(),
            _ => false,
        }
    }

    /// Returns the known value as an optional.
    pub fn known(&self) -> Option<&T> {
        match self {
            Self::Known(val) => Some(val),
            _ => None,
        }
    }

    /// Takes the value, replacing this instance with a default, empty instance.
    pub fn take(&mut self) -> Self {
        let mut taken = Self::default();
        std::mem::swap(self, &mut taken);
        taken
    }
}

impl<T> DiscoveryRequired<T>
where
    T: PartialEq + Ord,
{
    /// Checks if the value "can be" the given value.
    ///
    /// If the value is known, equality is used. If we only have a set of possible values, we check
    /// for set inclusion.
    pub fn can_be<'a, S>(&'a self, value: &'a S) -> bool
    where
        T: 'a,
        &'a T: PartialEq<&'a S>,
        T: Borrow<S>,
        S: Ord + ?Sized,
    {
        match self {
            Self::Known(known) => known == value,
            Self::PossibleValues(unknown) => unknown.contains(value),
        }
    }
}

impl<T> DiscoveryRequired<T>
where
    T: Ord,
{
    /// Records the value.
    ///
    /// The new value takes precedence. In other words, if `self` is a set of possible values and
    /// `other` is a known value, then the known value is returned.
    pub fn record(self, other: Self) -> Self {
        match (self, other) {
            (_, Self::Known(b)) => Self::Known(b),
            (Self::Known(a), Self::PossibleValues(_)) => Self::Known(a),
            (Self::PossibleValues(mut a), Self::PossibleValues(b)) => {
                a.extend(b);
                Self::PossibleValues(a)
            }
        }
    }

    /// Merges the value in.
    ///
    /// The two values have equal precedence. In other words, if `self` and `other` are both known
    /// values that are different from one another, then a set of possible values is returned.
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Known(a), Self::Known(b)) => {
                if a == b {
                    Self::Known(a)
                } else {
                    Self::PossibleValues(BTreeSet::from_iter([a, b]))
                }
            }
            (Self::Known(a), Self::PossibleValues(mut b))
            | (Self::PossibleValues(mut b), Self::Known(a)) => {
                b.insert(a);
                Self::PossibleValues(b)
            }
            (Self::PossibleValues(mut a), Self::PossibleValues(b)) => {
                a.extend(b);
                Self::PossibleValues(a)
            }
        }
    }

    /// Moves the known value, if any, to the possible values set.
    pub fn make_ambiguous(self) -> Self {
        match self {
            Self::Known(known) => Self::PossibleValues(BTreeSet::from_iter([known])),
            Self::PossibleValues(possible_values) => Self::PossibleValues(possible_values),
        }
    }
}

impl<T> Default for DiscoveryRequired<T>
where
    T: Ord,
{
    fn default() -> Self {
        Self::PossibleValues(BTreeSet::default())
    }
}

impl<T> From<T> for DiscoveryRequired<T>
where
    T: Ord,
{
    fn from(value: T) -> Self {
        Self::Known(value)
    }
}

/// Similar to [`DiscoveryRequired`], but allows a set of known values and possible values to be
/// recorded at the same time.
///
/// This type is basically a wrapper around two sets of value: one for known values and one for
/// possible values. It keeps the semantics of [`DiscoveryRequired`] in terms of recording and
/// merging.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryRequiredSet<T>
where
    T: Ord,
{
    known: BTreeSet<T>,
    #[serde(rename = "possibly_includes")]
    possible_values: BTreeSet<T>,
}

impl<T> DiscoveryRequiredSet<T>
where
    T: Ord,
{
    /// Checks if there are no values stored.
    pub fn is_empty(&self) -> bool {
        self.known.is_empty() && self.possible_values.is_empty()
    }

    /// Returns the set of known values.
    pub fn known(&self) -> &BTreeSet<T> {
        &self.known
    }

    /// Returns the set of possible values.
    pub fn possible_values(&self) -> &BTreeSet<T> {
        &self.possible_values
    }
}

impl<T> DiscoveryRequiredSet<T>
where
    T: Ord,
{
    /// Constructs a new set around the two sets.
    pub fn new<I, J>(known: I, possible_values: J) -> Self
    where
        I: IntoIterator<Item = T>,
        J: IntoIterator<Item = T>,
    {
        Self {
            known: known.into_iter().collect(),
            possible_values: possible_values.into_iter().collect(),
        }
    }

    /// Constructs a new set around a set of known values.
    pub fn from_known<I>(known: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self::new(known, [])
    }

    /// Constructs a new set around a set of possible values.
    pub fn from_possible_values<I>(possible_values: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self::new([], possible_values)
    }

    /// Records a known value.
    ///
    /// If the value was in the possible values set, it is removed.
    pub fn record_known(&mut self, value: T) {
        self.possible_values.remove(&value);
        self.known.insert(value);
    }

    /// Records a possible value.
    pub fn record_possible(&mut self, value: T) {
        if !self.known.contains(&value) {
            self.possible_values.insert(value);
        }
    }

    /// Removes a known value.
    ///
    /// If the value was in the possible values set, it is also removed.
    pub fn remove_known(&mut self, value: &T) {
        self.possible_values.remove(value);
        self.known.remove(value);
    }

    /// Downgrades a known value to a possible value, only if the value was in fact known.
    ///
    /// No change occurs if the value passed in is not in the known values set prior to this call.
    pub fn downgrade_to_possible_value(&mut self, value: T) {
        if self.known.remove(&value) {
            self.possible_values.insert(value);
        }
    }

    /// Moves all known values into the possible values set.
    pub fn make_ambiguous(mut self) -> Self {
        self.possible_values.extend(self.known);
        self.known = BTreeSet::default();
        self
    }
}

impl<T> DiscoveryRequiredSet<T>
where
    T: Clone + Ord,
{
    /// Merges the two sets.
    ///
    /// The new known value set is the intersection of the two known value sets. The new possible
    /// value sets is the union of all values, minus (set difference) known values (essentially a
    /// symmetric difference).
    pub fn merge(mut self, other: Self) -> Self {
        // Collect all possible values, before we lose information.
        self.possible_values.extend(self.known.iter().cloned());
        self.possible_values.extend(other.known.iter().cloned());
        self.possible_values.extend(other.possible_values);

        // Only keep values that we know exist on both sets.
        self.known = self
            .known
            .intersection(&other.known)
            .cloned()
            .collect::<BTreeSet<_>>();

        // Remove known values from possible values.
        self.possible_values = self
            .possible_values
            .difference(&self.known)
            .cloned()
            .collect::<BTreeSet<_>>();

        self
    }
}

#[cfg(test)]
mod discovery_test {
    use crate::discovery::{
        DiscoveryRequired,
        DiscoveryRequiredSet,
    };

    #[test]
    fn discovery_required_records_values() {
        assert_matches::assert_matches!(
            DiscoveryRequired::Known(12).record(DiscoveryRequired::Known(24)),
            DiscoveryRequired::Known(24)
        );
        assert_matches::assert_matches!(
            DiscoveryRequired::Known(12)
                .record(DiscoveryRequired::PossibleValues([1, 2, 3].into())),
            DiscoveryRequired::Known(12)
        );
        assert_matches::assert_matches!(
            DiscoveryRequired::PossibleValues([1, 2, 3].into())
                .record(DiscoveryRequired::PossibleValues([4, 5, 6].into())),
            DiscoveryRequired::PossibleValues(set) => {
                assert_eq!(set, [1,2,3,4,5,6].into());
            }
        );
        assert_matches::assert_matches!(
            DiscoveryRequired::PossibleValues([1, 2, 3].into())
                .record(DiscoveryRequired::Known(100)),
            DiscoveryRequired::Known(100)
        );
    }

    #[test]
    fn discovery_required_merges_values() {
        assert_matches::assert_matches!(
            DiscoveryRequired::Known(12).merge(DiscoveryRequired::Known(24)),
            DiscoveryRequired::PossibleValues(set) => {
                assert_eq!(set, [12, 24].into());
            }
        );
        assert_matches::assert_matches!(
            DiscoveryRequired::Known(12)
                .merge(DiscoveryRequired::PossibleValues([1, 2, 3].into())),
            DiscoveryRequired::PossibleValues(set) => {
                assert_eq!(set, [12, 1, 2, 3].into());
            }
        );
        assert_matches::assert_matches!(
            DiscoveryRequired::PossibleValues([1, 2, 3].into())
                .merge(DiscoveryRequired::PossibleValues([4, 5, 6].into())),
            DiscoveryRequired::PossibleValues(set) => {
                assert_eq!(set, [1, 2, 3, 4, 5, 6].into());
            }
        );
        assert_matches::assert_matches!(
            DiscoveryRequired::PossibleValues([1, 2, 3].into())
                .merge(DiscoveryRequired::Known(100)),
                DiscoveryRequired::PossibleValues(set) => {
                    assert_eq!(set, [100, 1, 2, 3].into());
                }
        );
    }

    #[test]
    fn discovery_required_set_records_values() {
        let mut set = DiscoveryRequiredSet::default();
        set.record_known(1);
        set.record_known(2);
        set.record_possible(3);
        set.record_possible(4);

        assert_eq!(set.known(), &[1, 2].into());
        assert_eq!(set.possible_values(), &[3, 4].into());

        set.record_known(3);

        assert_eq!(set.known(), &[1, 2, 3].into());
        assert_eq!(set.possible_values(), &[4].into());

        set.record_possible(3);

        assert_eq!(set.known(), &[1, 2, 3].into());
        assert_eq!(set.possible_values(), &[4].into());
    }

    #[test]
    fn discovery_required_set_merges_values() {
        assert_eq!(
            DiscoveryRequiredSet::new([1, 2, 3], [4, 5, 6])
                .clone()
                .merge(DiscoveryRequiredSet::new([7, 8, 9], [10, 11, 12])),
            DiscoveryRequiredSet::new([], [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
        );
        assert_eq!(
            DiscoveryRequiredSet::new([1, 2, 3], [4, 5, 6])
                .clone()
                .merge(DiscoveryRequiredSet::new([2, 3, 4], [5, 6, 7])),
            DiscoveryRequiredSet::new([2, 3], [1, 4, 5, 6, 7]),
        );
    }
}
