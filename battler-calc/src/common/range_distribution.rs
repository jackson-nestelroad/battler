use std::{
    fmt::Display,
    ops::{
        Add,
        Div,
        Mul,
        Sub,
    },
};

use crate::common::{
    Range,
    RangeValue,
};

/// A distribution of [`Range<I>`]s, on which mathematical operations can be performed to modify all
/// ranges in the distribution.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct RangeDistribution<I>(Vec<Range<I>>);

impl<I> RangeDistribution<I>
where
    I: RangeValue<I>,
{
    /// The length of the distribution.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Counts ranges that pass the filter.
    pub fn count<F>(&self, f: F) -> usize
    where
        F: Fn(&Range<I>) -> bool,
    {
        self.0.iter().cloned().filter(f).count()
    }

    /// The minimum value of all ranges.
    pub fn min(&self) -> Option<I> {
        self.0.iter().map(|range| range.a()).min()
    }

    /// The maximum value of all ranges.
    pub fn max(&self) -> Option<I> {
        self.0.iter().map(|range| range.b()).max()
    }

    /// Creates an iterator over the distribution.
    pub fn iter(&self) -> impl Iterator<Item = &Range<I>> {
        self.0.iter()
    }

    /// Creates an iterator over the distribution.
    pub fn into_iter(self) -> impl Iterator<Item = Range<I>> {
        self.0.into_iter()
    }
}

impl<I> From<Range<I>> for RangeDistribution<I> {
    fn from(value: Range<I>) -> Self {
        Self(Vec::from_iter([value]))
    }
}

impl<I> FromIterator<Range<I>> for RangeDistribution<I> {
    fn from_iter<T: IntoIterator<Item = Range<I>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<I> Display for RangeDistribution<I>
where
    I: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, range) in self.0.iter().enumerate() {
            write!(f, "{range}")?;
            if i > 0 {
                write!(f, ",")?;
            }
        }
        write!(f, "]")
    }
}

impl<I, T> Add<T> for RangeDistribution<I>
where
    I: RangeValue<I>,
    T: Clone,
    Range<I>: Add<T, Output = Range<I>>,
{
    type Output = Self;
    fn add(self, rhs: T) -> Self::Output {
        Self(self.0.into_iter().map(|val| val.add(rhs.clone())).collect())
    }
}

impl<I, T> Sub<T> for RangeDistribution<I>
where
    I: RangeValue<I>,
    T: Clone,
    Range<I>: Sub<T, Output = Range<I>>,
{
    type Output = Self;
    fn sub(self, rhs: T) -> Self::Output {
        Self(self.0.into_iter().map(|val| val.sub(rhs.clone())).collect())
    }
}

impl<I, T> Mul<T> for RangeDistribution<I>
where
    I: RangeValue<I>,
    T: Clone,
    Range<I>: Mul<T, Output = Range<I>>,
{
    type Output = Self;
    fn mul(self, rhs: T) -> Self::Output {
        Self(self.0.into_iter().map(|val| val.mul(rhs.clone())).collect())
    }
}

impl<I, T> Div<T> for RangeDistribution<I>
where
    I: RangeValue<I>,
    T: Clone,
    Range<I>: Div<T, Output = Range<I>>,
{
    type Output = Self;
    fn div(self, rhs: T) -> Self::Output {
        Self(self.0.into_iter().map(|val| val.div(rhs.clone())).collect())
    }
}

#[cfg(test)]
mod range_distribution_test {
    use crate::common::{
        Range,
        RangeDistribution,
    };

    #[test]
    fn performs_single_value_arithmetic() {
        let dist =
            RangeDistribution::from_iter([Range::new(0, 5), Range::new(6, 10), Range::new(8, 12)]);

        assert_eq!(
            dist.clone() + 2,
            RangeDistribution::from_iter([Range::new(2, 7), Range::new(8, 12), Range::new(10, 14)])
        );
        assert_eq!(
            dist.clone() - 10,
            RangeDistribution::from_iter([
                Range::new(-10, -5),
                Range::new(-4, 0),
                Range::new(-2, 2)
            ])
        );
        assert_eq!(
            dist.clone() * 3,
            RangeDistribution::from_iter([
                Range::new(0, 15),
                Range::new(18, 30),
                Range::new(24, 36)
            ])
        );
        assert_eq!(
            dist.clone() / 2,
            RangeDistribution::from_iter([Range::new(0, 2), Range::new(3, 5), Range::new(4, 6)])
        );
    }

    #[test]
    fn performs_range_arithmetic() {
        let dist =
            RangeDistribution::from_iter([Range::new(0, 5), Range::new(6, 10), Range::new(8, 12)]);

        assert_eq!(
            dist.clone() + Range::new(1, 10),
            RangeDistribution::from_iter([Range::new(1, 15), Range::new(7, 20), Range::new(9, 22)])
        );
        assert_eq!(
            dist.clone() - Range::new(2, 3),
            RangeDistribution::from_iter([Range::new(-3, 3), Range::new(3, 8), Range::new(5, 10)])
        );
        assert_eq!(
            dist.clone() * Range::new(2, 4),
            RangeDistribution::from_iter([
                Range::new(0, 20),
                Range::new(12, 40),
                Range::new(16, 48)
            ])
        );
        assert_eq!(
            dist.clone() / Range::new(1, 2),
            RangeDistribution::from_iter([Range::new(0, 5), Range::new(3, 10), Range::new(4, 12)])
        );
    }

    #[test]
    fn counts_ranges_matching_filter() {
        let dist = RangeDistribution::from_iter([
            Range::new(0, 5),
            Range::new(6, 10),
            Range::new(8, 12),
            Range::new(20, 40),
        ]);

        assert_eq!(dist.len(), 4);
        assert_eq!(dist.count(|range| range.contains(5)), 1);
        assert_eq!(dist.count(|range| range.contains(10)), 2);
        assert_eq!(dist.count(|range| range.contains(100)), 0);
    }
}
