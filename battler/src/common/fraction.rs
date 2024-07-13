use std::{
    cmp,
    fmt::{
        self,
        Display,
    },
    marker::PhantomData,
    ops::{
        Add,
        Div,
        Mul,
        Sub,
    },
    str::FromStr,
};

use num::{
    integer::Roots,
    traits::{
        WrappingAdd,
        WrappingMul,
        WrappingSub,
    },
    FromPrimitive,
    Integer,
    PrimInt,
};
use serde::{
    de::{
        Unexpected,
        Visitor,
    },
    Deserialize,
    Serialize,
    Serializer,
};

use crate::common::{
    Error,
    WrapResultError,
};

/// An integer type that can be used as the inner type of [`Fraction`].
pub trait FractionInteger: Integer + FromPrimitive + Copy {}
impl<I> FractionInteger for I where I: Integer + FromPrimitive + Copy {}

/// A fraction, usable in calculations.
///
/// A fraction is serializable as:
/// - A fraction string (`"1/2"`).
/// - An integer (`20`), which represents an integer (demoninator == 1).
/// - A floating point number (`1.5`), which is converted to a fraction out of 4096.
/// - A percentage string (`"60%"`).
/// - A two-length array (`[2,5]`).
#[derive(Debug, Clone, Copy)]
pub struct Fraction<I> {
    num: I,
    den: I,
}

impl<I> Fraction<I>
where
    I: FractionInteger,
{
    /// Creates a new fraction.
    pub fn new(n: I, d: I) -> Self {
        Self { num: n, den: d }
    }

    /// Creates a new percentage as a fraction.
    pub fn percentage(n: I) -> Self {
        Fraction {
            num: n,
            den: I::from_u8(100).unwrap(),
        }
        .simplify()
    }

    /// Creates a new fraction from an [`f64`].
    ///
    /// Flatoing point precision is preserved by creating a fraction with a denominator of 4096.
    pub fn from_f64(value: f64) -> Self {
        let num = I::from_f64(value * 4096f64).unwrap();
        Self::new(num, I::from_u16(4096).unwrap()).simplify()
    }

    /// The numerator of the fraction.
    pub fn numerator(&self) -> I {
        self.num
    }

    /// The denominator of the fraction.
    ///
    /// A flat percentage is always out of 100.
    pub fn denominator(&self) -> I {
        self.den
    }

    /// Is the fraction whole (i.e., an integer)?
    pub fn is_whole(&self) -> bool {
        self.den == I::one()
    }

    /// Simplifies the fraction.
    pub fn simplify(&self) -> Self {
        let n = self.numerator();
        let d = self.denominator();
        let gcd = n.gcd(&d);
        Fraction::new(n.div(gcd), d.div(gcd))
    }

    /// Returns the floored integer representation of the fraction.
    ///
    /// The integer will be truncated, as if performing integer division.
    pub fn floor(&self) -> I {
        self.numerator().div(self.denominator())
    }

    /// Returns the ceiled integer representation of the fraction.
    pub fn ceil(&self) -> I {
        num::Integer::div_ceil(&self.numerator(), &self.denominator())
    }

    /// Returns the rounded integer representation of the fraction.
    pub fn round(&self) -> I
    where
        I: PrimInt,
    {
        (self.numerator().add(self.denominator().shr(1))).div(self.denominator())
    }

    /// Converts the [`Fraction<I>`] to a [`Fraction<T>`], given that `T: From<I>`.
    pub fn convert<T>(self) -> Fraction<T>
    where
        T: FractionInteger + From<I>,
    {
        Fraction::new(T::from(self.numerator()), T::from(self.denominator()))
    }

    /// Attempts converting the [`Fraction<I>`] to a [`Fraction<T>`], given that `T: TryFrom<I>`.
    pub fn try_convert<T>(self) -> Result<Fraction<T>, T::Error>
    where
        T: FractionInteger + TryFrom<I>,
    {
        Ok(Fraction::new(
            T::try_from(self.numerator())?,
            T::try_from(self.denominator())?,
        ))
    }

    /// Returns the inverse of this fraction.
    pub fn inverse(&self) -> Self {
        Self::new(self.denominator(), self.numerator())
    }

    fn normalize(a: &Fraction<I>, b: &Fraction<I>) -> (Fraction<I>, Fraction<I>) {
        let a1 = a.numerator();
        let a2 = a.denominator();
        let b1 = b.numerator();
        let b2 = b.denominator();
        // Note: This calculation could overflow if the denominators are large enough.
        let lcm = a2.lcm(&b2);
        let a_mul = lcm.div(a2);
        let b_mul = lcm.div(b2);
        // Note: This calculation could overflow if the numerators are large enough.
        (
            Fraction::new(a1.mul(a_mul), lcm),
            Fraction::new(b1.mul(b_mul), lcm),
        )
    }
}

impl<I> Fraction<I>
where
    I: FractionInteger + WrappingAdd,
{
    /// Wrapping addition.
    pub fn wrapping_add(&self, rhs: &Self) -> Self {
        let (lhs, rhs) = Self::normalize(&self, &rhs);
        Self::new(
            lhs.numerator().wrapping_add(&rhs.numerator()),
            lhs.denominator(),
        )
    }
}

impl<I> Fraction<I>
where
    I: FractionInteger + WrappingSub,
{
    /// Wrapping subtraction.
    pub fn wrapping_sub(&self, rhs: &Self) -> Self {
        let (lhs, rhs) = Self::normalize(&self, &rhs);
        Self::new(
            lhs.numerator().wrapping_sub(&rhs.numerator()),
            lhs.denominator(),
        )
    }
}

impl<I> Fraction<I>
where
    I: FractionInteger + WrappingMul,
{
    /// Wrapping multiplication.
    pub fn wrapping_mul(&self, rhs: &Self) -> Self {
        Self::new(
            self.numerator().wrapping_mul(&rhs.numerator()),
            self.denominator().wrapping_mul(&rhs.denominator()),
        )
        .simplify()
    }
}

impl<I> Fraction<I>
where
    I: FractionInteger + Roots,
{
    pub fn sqrt(&self) -> Self {
        Self::new(self.numerator().sqrt(), self.denominator().sqrt()).simplify()
    }
}

impl<I> Display for Fraction<I>
where
    I: FractionInteger + Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == I::one() {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

impl<I> From<I> for Fraction<I>
where
    I: FractionInteger,
{
    fn from(value: I) -> Self {
        Self::new(value, I::one())
    }
}

impl<I> FromStr for Fraction<I>
where
    I: FractionInteger + FromStr + Display,
    <I as FromStr>::Err: Display,
{
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((n, d)) = s.split_once('/') {
            let n = n
                .parse()
                .wrap_error_with_format(format_args!("invalid numerator: {n}"))?;
            let d = d
                .parse()
                .wrap_error_with_format(format_args!("invalid denominator: {n}"))?;
            Ok(Self::new(n, d))
        } else {
            let s = match s.strip_suffix('%') {
                Some(s) => s,
                None => s,
            };
            Ok(Self::percentage(s.parse().wrap_error_with_format(
                format_args!("invalid percentage: {s}"),
            )?))
        }
    }
}

impl<I> PartialEq for Fraction<I>
where
    I: FractionInteger,
{
    fn eq(&self, other: &Self) -> bool {
        let (a, b) = Self::normalize(self, other);
        a.numerator().eq(&b.numerator())
    }
}

impl<I> Eq for Fraction<I> where I: FractionInteger {}

impl<I> PartialEq<I> for Fraction<I>
where
    I: FractionInteger,
{
    fn eq(&self, other: &I) -> bool {
        self.eq(&Fraction::from(*other))
    }
}

impl<I> Ord for Fraction<I>
where
    I: FractionInteger,
{
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        let (a, b) = Self::normalize(self, other);
        a.numerator().cmp(&b.numerator())
    }
}

impl<I> PartialOrd for Fraction<I>
where
    I: FractionInteger,
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<I> PartialOrd<I> for Fraction<I>
where
    I: FractionInteger,
{
    fn partial_cmp(&self, other: &I) -> Option<cmp::Ordering> {
        self.partial_cmp(&Fraction::from(*other))
    }
}

impl<I> Add<I> for Fraction<I>
where
    I: FractionInteger,
{
    type Output = Fraction<I>;
    fn add(self, rhs: I) -> Self::Output {
        Self::Output::new(
            self.numerator().add(rhs.mul(self.denominator())),
            self.denominator(),
        )
        .simplify()
    }
}

impl<I> Add<Fraction<I>> for Fraction<I>
where
    I: FractionInteger,
{
    type Output = Fraction<I>;
    fn add(self, rhs: Fraction<I>) -> Self::Output {
        let (lhs, rhs) = Self::normalize(&self, &rhs);
        Self::Output::new(lhs.numerator().add(rhs.numerator()), lhs.denominator())
    }
}

impl<I> Sub<I> for Fraction<I>
where
    I: FractionInteger,
{
    type Output = Fraction<I>;
    fn sub(self, rhs: I) -> Self::Output {
        Self::Output::new(
            self.numerator().sub(rhs.mul(self.denominator())),
            self.denominator(),
        )
        .simplify()
    }
}

impl<I> Sub<Fraction<I>> for Fraction<I>
where
    I: FractionInteger,
{
    type Output = Fraction<I>;
    fn sub(self, rhs: Fraction<I>) -> Self::Output {
        let (lhs, rhs) = Self::normalize(&self, &rhs);
        Self::Output::new(lhs.numerator().sub(rhs.numerator()), lhs.denominator())
    }
}

impl<I> Mul<I> for Fraction<I>
where
    I: FractionInteger,
{
    type Output = Fraction<I>;
    fn mul(self, rhs: I) -> Self::Output {
        Self::Output::new(self.numerator().mul(rhs), self.denominator()).simplify()
    }
}

impl<I> Mul<Fraction<I>> for Fraction<I>
where
    I: FractionInteger,
{
    type Output = Fraction<I>;
    fn mul(self, rhs: Fraction<I>) -> Self::Output {
        Self::Output::new(
            self.numerator().mul(rhs.numerator()),
            self.denominator().mul(rhs.denominator()),
        )
        .simplify()
    }
}

impl<I> Div<I> for Fraction<I>
where
    I: FractionInteger,
{
    type Output = Fraction<I>;
    fn div(self, rhs: I) -> Self::Output {
        self.mul(Fraction::new(I::one(), rhs))
    }
}

impl<I> Div<Fraction<I>> for Fraction<I>
where
    I: FractionInteger,
{
    type Output = Fraction<I>;
    fn div(self, rhs: Fraction<I>) -> Self::Output {
        self.mul(rhs.inverse())
    }
}

impl<I> Serialize for Fraction<I>
where
    I: FractionInteger + Into<u64> + Display,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.is_whole() {
            serializer.serialize_u64(self.floor().into())
        } else {
            serializer.serialize_str(&format!("{self}"))
        }
    }
}

struct FractionVisitor<I> {
    _phantom: PhantomData<I>,
}

impl<I> FractionVisitor<I>
where
    I: Integer,
{
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<'de> Visitor<'de> for FractionVisitor<u16> {
    type Value = Fraction<u16>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "an integer, a fraction string, a percentage string, or an array of 2 integers"
        )
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v as u16))
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v as u16))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v as u16))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from_f64(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Self::Value::from_str(v).map_err(|_| E::invalid_value(Unexpected::Str(&v), &self))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let num = match seq.next_element()? {
            Some(v) => v,
            None => return Err(serde::de::Error::invalid_length(0, &self)),
        };
        let den = match seq.next_element()? {
            Some(v) => v,
            None => return Err(serde::de::Error::invalid_length(1, &self)),
        };
        if seq.next_element::<u8>()?.is_some() {
            return Err(serde::de::Error::invalid_length(3, &self));
        }
        Ok(Self::Value::new(num, den))
    }
}

impl<'de> Deserialize<'de> for Fraction<u16> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(FractionVisitor::<u16>::new())
    }
}

#[cfg(test)]
mod percentage_tests {
    use crate::common::{
        test_deserialization,
        test_serialization,
        Fraction,
    };

    #[test]
    fn serializes_to_string() {
        test_serialization(Fraction::percentage(25), "\"1/4\"");
        test_serialization(Fraction::percentage(100), "1");
        test_serialization(Fraction::new(1, 2), "\"1/2\"");
        test_serialization(Fraction::new(1, 3), "\"1/3\"");
        test_serialization(Fraction::new(20, 147), "\"20/147\"");
    }

    #[test]
    fn deserializes_integers() {
        test_deserialization("25", Fraction::new(25, 1));
        test_deserialization("77", Fraction::new(77, 1));
        test_deserialization("100", Fraction::new(100, 1));
    }

    #[test]
    fn deserializes_floats() {
        test_deserialization("2.5", Fraction::new(5, 2));
        test_deserialization("1.33", Fraction::new(5447, 4096));
        test_deserialization("10.0", Fraction::new(10, 1));
    }

    #[test]
    fn deserializes_flat_percentages() {
        test_deserialization("\"25%\"", Fraction::new(1, 4));
        test_deserialization("\"77%\"", Fraction::new(77, 100));
        test_deserialization("\"100%\"", Fraction::new(1, 1));
    }

    #[test]
    fn deserializes_fraction_arrays() {
        test_deserialization("[1,2]", Fraction::new(1, 2));
        test_deserialization("[33, 100]", Fraction::new(33, 100));
    }

    #[test]
    fn percentage_equality() {
        assert_eq!(Fraction::percentage(10), Fraction::percentage(10));
        assert_eq!(Fraction::percentage(20), Fraction::new(1, 5));
        assert_eq!(Fraction::new(35, 100), Fraction::percentage(35));
        assert_eq!(Fraction::new(3, 4), Fraction::new(12, 16));
    }

    #[test]
    fn percentage_inequality() {
        assert_ne!(Fraction::percentage(10), Fraction::percentage(100));
        assert_ne!(Fraction::percentage(20), Fraction::new(1, 20));
        assert_ne!(Fraction::new(35, 100), Fraction::percentage(12));
        assert_ne!(Fraction::new(3, 4), Fraction::new(3, 5));
    }

    #[test]
    fn percentage_ordering() {
        let mut percentages = vec![
            Fraction::new(3, 4),
            Fraction::new(3, 200),
            Fraction::percentage(1),
            Fraction::new(2, 7),
            Fraction::new(2, 100),
            Fraction::percentage(100),
            Fraction::new(1, 4),
            Fraction::new(1, 2),
            Fraction::percentage(60),
        ];
        percentages.sort();
        pretty_assertions::assert_eq!(
            percentages,
            vec![
                Fraction::percentage(1),
                Fraction::new(3, 200),
                Fraction::new(2, 100),
                Fraction::new(1, 4),
                Fraction::new(2, 7),
                Fraction::new(1, 2),
                Fraction::percentage(60),
                Fraction::new(3, 4),
                Fraction::percentage(100),
            ]
        );
    }

    #[test]
    fn floor_division() {
        assert_eq!(Fraction::percentage(1).floor(), 0);
        assert_eq!(Fraction::new(77, 12).floor(), 6);
        assert_eq!(Fraction::percentage(2500).floor(), 25);
        assert_eq!(Fraction::new(33, 15).floor(), 2);
        assert_eq!(Fraction::new(1020, 25).floor(), 40);
        assert_eq!(Fraction::new(1, 2).floor(), 0);
    }

    #[test]
    fn round_division() {
        assert_eq!(Fraction::percentage(1).round(), 0);
        assert_eq!(Fraction::new(77, 12).round(), 6);
        assert_eq!(Fraction::percentage(2500).round(), 25);
        assert_eq!(Fraction::new(33, 15).round(), 2);
        assert_eq!(Fraction::new(1020, 25).round(), 41);

        assert_eq!(Fraction::new(1, 2).round(), 1);
        assert_eq!(Fraction::new(2, 2).round(), 1);
        assert_eq!(Fraction::new(3, 2).round(), 2);
        assert_eq!(Fraction::new(4, 2).round(), 2);

        assert_eq!(Fraction::new(1, 7).round(), 0);
        assert_eq!(Fraction::new(2, 7).round(), 0);
        assert_eq!(Fraction::new(3, 7).round(), 0);
        assert_eq!(Fraction::new(4, 7).round(), 1);
        assert_eq!(Fraction::new(5, 7).round(), 1);
        assert_eq!(Fraction::new(6, 7).round(), 1);
        assert_eq!(Fraction::new(7, 7).round(), 1);
        assert_eq!(Fraction::new(8, 7).round(), 1);
    }

    #[test]
    fn ceil_division() {
        assert_eq!(Fraction::percentage(1).ceil(), 1);
        assert_eq!(Fraction::new(77, 12).ceil(), 7);
        assert_eq!(Fraction::percentage(2500).ceil(), 25);
    }

    #[test]
    fn integer_addition() {
        assert_eq!(Fraction::percentage(1) + 10000, Fraction::new(1000001, 100));
        assert_eq!(Fraction::new(12, 77) + 2, Fraction::new(166, 77));
        assert_eq!(Fraction::percentage(25) + 0, Fraction::new(1, 4));
    }

    #[test]
    fn fraction_addition() {
        assert_eq!(
            Fraction::new(12, 77) + Fraction::new(5, 6),
            Fraction::new(457, 462)
        );
        assert_eq!(
            Fraction::new(12, 12) + Fraction::new(53, 53),
            Fraction::from(2)
        );
        assert_eq!(
            Fraction::new(1, 4) + Fraction::new(2, 4),
            Fraction::new(3, 4)
        );
    }

    #[test]
    fn integer_subtraction() {
        assert_eq!(Fraction::percentage(1) - 10000, Fraction::new(-999999, 100));
        assert_eq!(Fraction::new(2000, 77) - 2, Fraction::new(1846, 77));
        assert_eq!(Fraction::percentage(25) - 0, Fraction::new(1, 4));
    }

    #[test]
    fn fraction_subtraction() {
        assert_eq!(
            Fraction::new(12, 77) - Fraction::new(5, 6),
            Fraction::new(-313, 462)
        );
        assert_eq!(
            Fraction::new(12, 12) - Fraction::new(53, 53),
            Fraction::from(0)
        );
        assert_eq!(
            Fraction::new(2, 4) - Fraction::new(1, 4),
            Fraction::new(1, 4)
        );
    }

    #[test]
    fn integer_multiplication() {
        assert_eq!(Fraction::percentage(1) * 10000, Fraction::from(100));
        assert_eq!(Fraction::new(12, 77) * 85, Fraction::new(1020, 77));
        assert_eq!(Fraction::percentage(25) * 100, Fraction::from(25));
        assert_eq!(Fraction::percentage(25) * 1, Fraction::new(1, 4));
        assert_eq!(Fraction::new(10, 50) * 2, Fraction::new(2, 5));
    }

    #[test]
    fn fraction_multiplication() {
        assert_eq!(
            Fraction::new(12, 77) * Fraction::new(5, 6),
            Fraction::new(10, 77)
        );
        assert_eq!(
            Fraction::new(12, 12) * Fraction::new(53, 53),
            Fraction::from(1)
        );
        assert_eq!(
            Fraction::new(1, 4) * Fraction::new(2, 4),
            Fraction::new(1, 8)
        );
    }

    #[test]
    fn integer_division() {
        assert_eq!(Fraction::percentage(1) / 10000, Fraction::new(1, 1000000));
        assert_eq!(Fraction::new(12, 77) / 85, Fraction::new(12, 6545));
        assert_eq!(Fraction::percentage(25) / 100, Fraction::new(1, 400));
        assert_eq!(Fraction::percentage(25) / 1, Fraction::new(1, 4));
        assert_eq!(Fraction::new(10, 50) / 2, Fraction::new(1, 10));
    }

    #[test]
    fn fraction_division() {
        assert_eq!(
            Fraction::new(12, 77) / Fraction::new(5, 6),
            Fraction::new(72, 385)
        );
        assert_eq!(
            Fraction::new(12, 12) / Fraction::new(53, 53),
            Fraction::from(1)
        );
        assert_eq!(
            Fraction::new(1, 4) / Fraction::new(2, 4),
            Fraction::new(1, 2)
        );
    }
}
