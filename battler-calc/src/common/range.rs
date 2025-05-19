use std::{
    cmp::Ordering,
    fmt::Display,
    ops::{
        Add,
        Div,
        Mul,
        Sub,
    },
};

use num::{
    Integer,
    traits::{
        SaturatingAdd,
        SaturatingMul,
        SaturatingSub,
    },
};

/// An integer type that can be used as the inner type of [`Range`].
pub trait RangeValue<I>:
    Div<I, Output = I>
    + SaturatingAdd
    + SaturatingSub
    + SaturatingMul
    + Clone
    + Copy
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
{
}
impl<I> RangeValue<I> for I where
    I: Div<I, Output = I>
        + SaturatingAdd
        + SaturatingSub
        + SaturatingMul
        + Clone
        + Copy
        + PartialEq
        + Eq
        + PartialOrd
        + Ord
{
}

/// A range of integers, on which mathematical operations can be performed to modify the range.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Range<I>(I, I);

impl<I> Range<I>
where
    I: RangeValue<I>,
{
    /// Creates a new range.
    pub fn new(a: I, b: I) -> Self {
        assert!(a <= b, "range start exceeds range end");
        Self(a, b)
    }

    /// The start of the range.
    pub fn a(&self) -> I {
        self.0
    }

    /// The end of the range (inclusive).
    pub fn b(&self) -> I {
        self.1
    }

    /// Ceiled integer division.
    pub fn div_ceil(self, rhs: I) -> Self
    where
        I: Integer,
    {
        Self::new(self.a().div_ceil(&rhs), self.b().div_ceil(&rhs))
    }

    /// Checks if a value is in range.
    pub fn contains(&self, v: I) -> bool {
        v >= self.a() && v <= self.b()
    }

    /// Performs a strict comparison, checking if all values in this range is less than
    /// or greater than another range.
    pub fn strict_cmp(&self, rhs: &Self) -> Option<Ordering> {
        if self.a() < rhs.a() && self.b() < rhs.a() {
            Some(Ordering::Less)
        } else if self.a() > rhs.b() && self.b() > rhs.b() {
            Some(Ordering::Greater)
        } else if self.a() == rhs.a() && self.b() == rhs.b() {
            Some(Ordering::Equal)
        } else {
            None
        }
    }

    /// Checks if this range overlaps with another.
    pub fn overlaps(&self, rhs: &Self) -> bool {
        match self.strict_cmp(rhs) {
            Some(Ordering::Less | Ordering::Greater) => false,
            _ => true,
        }
    }

    /// Maps the bounds of the range to values of a different a type.
    pub fn map<F, T>(&self, f: F) -> Range<T>
    where
        F: Fn(I) -> T,
        T: RangeValue<T>,
    {
        Range::new(f(self.a()), f(self.b()))
    }
}

// Do not allow a range to be the inner value of a range. This is so we can use a negative impl to
// overload the arithmetic operators for values AND ranges.
impl<I> !RangeValue<I> for Range<I> {}

impl<I> From<I> for Range<I>
where
    I: RangeValue<I>,
{
    fn from(value: I) -> Self {
        Self::new(value, value)
    }
}

impl<I> Display for Range<I>
where
    I: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{},{}]", self.0, self.1)
    }
}

impl<I> Add for Range<I>
where
    I: RangeValue<I>,
{
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(
            self.a().saturating_add(&rhs.a()),
            self.b().saturating_add(&rhs.b()),
        )
    }
}

impl<I, T> Add<T> for Range<I>
where
    I: RangeValue<I>,
    T: RangeValue<T> + Into<I> + Clone,
{
    type Output = Self;
    fn add(self, rhs: T) -> Self::Output {
        self.add(Range::from(rhs.into()))
    }
}

impl<I> Sub for Range<I>
where
    I: RangeValue<I>,
{
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(
            self.a().saturating_sub(&rhs.b()),
            self.b().saturating_sub(&rhs.a()),
        )
    }
}

impl<I, T> Sub<T> for Range<I>
where
    I: RangeValue<I>,
    T: RangeValue<T> + Into<I>,
{
    type Output = Self;
    fn sub(self, rhs: T) -> Self::Output {
        self.sub(Range::from(rhs.into()))
    }
}

impl<I> Mul for Range<I>
where
    I: RangeValue<I>,
{
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(
            self.a().saturating_mul(&rhs.a()),
            self.b().saturating_mul(&rhs.b()),
        )
    }
}

impl<I, T> Mul<T> for Range<I>
where
    I: RangeValue<I>,
    T: RangeValue<T> + Into<I> + Clone,
{
    type Output = Self;
    fn mul(self, rhs: T) -> Self::Output {
        self.mul(Range::from(rhs.into()))
    }
}

impl<I> Div for Range<I>
where
    I: RangeValue<I>,
{
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.a().div(rhs.b()), self.b().div(rhs.a()))
    }
}

impl<I, T> Div<T> for Range<I>
where
    I: RangeValue<I> + Div<T, Output = I>,
    T: RangeValue<T>,
{
    type Output = Self;
    fn div(self, rhs: T) -> Self::Output {
        Self::new(self.a().div(rhs), self.b().div(rhs))
    }
}

#[cfg(test)]
mod range_test {
    use std::cmp::Ordering;

    use crate::common::Range;

    #[test]
    fn performs_single_value_arithmetic() {
        let range = Range::new(1, 10);

        assert_eq!(range + 1, Range::new(2, 11));
        assert_eq!(range - 10, Range::new(-9, 0));
        assert_eq!(range * 6, Range::new(6, 60));
        assert_eq!(range / 2, Range::new(0, 5));
    }

    #[test]
    fn performs_range_arithmetic() {
        let range = Range::new(1, 10);

        assert_eq!(range + Range::new(5, 8), Range::new(6, 18));
        assert_eq!(range - Range::new(0, 2), Range::new(-1, 10));
        assert_eq!(range - Range::new(10, 10), Range::new(-9, 0));
        assert_eq!(range * Range::new(2, 4), Range::new(2, 40));
        assert_eq!(range / Range::new(1, 5), Range::new(0, 10));
        assert_eq!(range / Range::new(20, 25), Range::new(0, 0));
    }

    #[test]
    fn tests_values_in_range() {
        assert!(Range::new(1, 10).contains(1));
        assert!(Range::new(1, 10).contains(5));
        assert!(Range::new(1, 10).contains(10));
        assert!(!Range::new(1, 10).contains(0));
        assert!(!Range::new(1, 10).contains(11));
    }

    #[test]
    fn strict_compares_ranges() {
        assert_matches::assert_matches!(Range::new(1, 10).strict_cmp(&Range::new(5, 15)), None);
        assert_matches::assert_matches!(Range::new(1, 10).strict_cmp(&Range::new(-5, 1)), None);
        assert_matches::assert_matches!(
            Range::new(1, 10).strict_cmp(&Range::new(-5, 0)),
            Some(Ordering::Greater)
        );
        assert_matches::assert_matches!(
            Range::new(1, 10).strict_cmp(&Range::new(11, 20)),
            Some(Ordering::Less)
        );
        assert_matches::assert_matches!(
            Range::new(1, 10).strict_cmp(&Range::new(1, 10)),
            Some(Ordering::Equal)
        );
    }

    #[test]
    fn tests_range_overlap() {
        assert!(Range::new(1, 10).overlaps(&Range::new(5, 15)));
        assert!(Range::new(1, 10).overlaps(&Range::new(-5, 1)));
        assert!(!Range::new(1, 10).overlaps(&Range::new(-5, 0)));
        assert!(!Range::new(1, 10).overlaps(&Range::new(11, 20)));
        assert!(Range::new(1, 10).overlaps(&Range::new(1, 10)));
    }
}
