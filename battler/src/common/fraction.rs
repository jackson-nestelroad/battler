use std::{
    cmp,
    fmt,
    fmt::Display,
    ops::{
        Div,
        Mul,
    },
    str::FromStr,
};

use num::Integer;
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

/// A fraction, usable in calculations.
///
/// A fraction is serializable as:
/// - A fraction string (`"1/2"`).
/// - An integer (`20`), which represents an integer (demoninator == 1).
/// - A floating point number (`1.5`), which is converted to a fraction out of 4096.
/// - A percentage string (`"60%"`).
/// - A two-length array (`[2,5]`).
#[derive(Debug, Clone)]
pub struct Fraction {
    num: u32,
    den: u32,
}

impl Fraction {
    /// Creates a new fraction.
    pub fn new(n: u32, d: u32) -> Fraction {
        Fraction { num: n, den: d }
    }

    /// Creates a new percentage as a fraction.
    pub fn percentage(n: u32) -> Fraction {
        Fraction { num: n, den: 100 }.simplify()
    }

    /// The numerator of the fraction.
    pub fn numerator(&self) -> u32 {
        self.num
    }

    /// The denominator of the fraction.
    ///
    /// A flat percentage is always out of 100.
    pub fn denominator(&self) -> u32 {
        self.den
    }

    /// Is the fraction whole (i.e., an integer)?
    pub fn is_whole(&self) -> bool {
        self.den == 1
    }

    /// Simplifies the fraction.
    pub fn simplify(&self) -> Fraction {
        let n = self.numerator();
        let d = self.denominator();
        let gcd = n.gcd(&d);
        Fraction::new(n.div(gcd), d.div(gcd))
    }

    /// Returns the integer representation of the percentage.
    ///
    /// The integer will be truncated, as if performing integer division.
    pub fn integer(&self) -> u32 {
        self.numerator().div(self.denominator())
    }

    fn normalize(a: &Fraction, b: &Fraction) -> (Fraction, Fraction) {
        let a1 = a.numerator();
        let a2 = a.denominator();
        let b1 = b.numerator();
        let b2 = b.denominator();
        let lcm = a2.lcm(&b2);
        let a_mul = lcm.div(a2);
        let b_mul = lcm.div(b2);
        (
            Fraction::new(a1.mul(a_mul), lcm),
            Fraction::new(b1.mul(b_mul), lcm),
        )
    }
}

impl Display for Fraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

impl From<u32> for Fraction {
    fn from(value: u32) -> Self {
        Self::new(value, 1)
    }
}

impl From<f64> for Fraction {
    fn from(value: f64) -> Self {
        Self::new((value * 4096f64).trunc() as u32, 4096).simplify()
    }
}

impl FromStr for Fraction {
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

impl Into<u32> for Fraction {
    fn into(self) -> u32 {
        self.integer()
    }
}

impl PartialEq for Fraction {
    fn eq(&self, other: &Self) -> bool {
        let (a, b) = Self::normalize(self, other);
        a.numerator().eq(&b.numerator())
    }
}

impl Eq for Fraction {}

impl Ord for Fraction {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        let (a, b) = Self::normalize(self, other);
        a.numerator().cmp(&b.numerator())
    }
}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Mul<u32> for &Fraction {
    type Output = Fraction;
    fn mul(self, rhs: u32) -> Self::Output {
        Self::Output::new(self.numerator().mul(rhs), self.denominator()).simplify()
    }
}

impl Mul<u32> for Fraction {
    type Output = Self;
    fn mul(self, rhs: u32) -> Self::Output {
        Mul::mul(&self, rhs)
    }
}

impl Mul<&Fraction> for &Fraction {
    type Output = Fraction;
    fn mul(self, rhs: &Fraction) -> Self::Output {
        Self::Output::new(
            self.numerator().mul(rhs.numerator()),
            self.denominator().mul(rhs.denominator()),
        )
        .simplify()
    }
}

impl Mul<Fraction> for &Fraction {
    type Output = Fraction;
    fn mul(self, rhs: Fraction) -> Self::Output {
        Mul::mul(self, &rhs)
    }
}

impl Mul<&Fraction> for Fraction {
    type Output = Self;
    fn mul(self, rhs: &Fraction) -> Self::Output {
        Mul::mul(&self, rhs)
    }
}

impl Mul<Fraction> for Fraction {
    type Output = Self;
    fn mul(self, rhs: Fraction) -> Self::Output {
        Mul::mul(&self, &rhs)
    }
}

impl Serialize for Fraction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.is_whole() {
            serializer.serialize_u32(self.integer())
        } else {
            serializer.serialize_str(&format!("{self}"))
        }
    }
}

struct FractionVisitor;

impl<'de> Visitor<'de> for FractionVisitor {
    type Value = Fraction;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "an integer, a fraction string, a percentage string, or an array of 2 integers"
        )
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v as u32))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v))
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

impl<'de> Deserialize<'de> for Fraction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(FractionVisitor)
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
    fn integer_multiplication() {
        assert_eq!(Fraction::percentage(1) * 10000, 100.into());
        assert_eq!(Fraction::new(12, 77) * 85, Fraction::new(1020, 77));
        assert_eq!(Fraction::percentage(25) * 100, 25.into());
        assert_eq!(Fraction::percentage(25) * 1, Fraction::new(1, 4));
        assert_eq!(Fraction::new(10, 50) * 2, Fraction::new(2, 5));
    }

    #[test]
    fn fraction_multiplication() {
        assert_eq!(
            Fraction::new(12, 77) * Fraction::new(5, 6),
            Fraction::new(10, 77)
        );
        assert_eq!(Fraction::new(12, 12) * Fraction::new(53, 53), 1.into());
        assert_eq!(
            Fraction::new(1, 4) * Fraction::new(2, 4),
            Fraction::new(1, 8)
        );
    }

    #[test]
    fn integer_conversion() {
        assert_eq!(Fraction::new(33, 15).integer(), 2);
        assert_eq!(Fraction::new(1020, 25).integer(), 40);
        assert_eq!(Fraction::new(1, 2).integer(), 0);
    }
}
