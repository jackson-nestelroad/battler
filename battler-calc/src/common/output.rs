use std::{
    fmt::Display,
    ops::{
        Add,
        Div,
        Mul,
        Sub,
    },
};

/// An output value with a description of each mathematical operation performed on it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Output<T> {
    value: T,
    description: Vec<String>,
}

impl<T> Output<T>
where
    T: Default,
{
    pub fn new<V, I, S>(val: V, description: I) -> Self
    where
        V: Into<T>,
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            value: val.into(),
            description: description
                .into_iter()
                .map(|reason| reason.into())
                .collect(),
        }
    }

    /// Starts the output with a new value with an attached message.
    pub fn start<V, S>(val: V, reason: S) -> Self
    where
        V: Clone + Display + Into<T>,
        S: Display,
    {
        let mut s = Self::default();
        s.set(val, reason);
        s
    }

    /// The current value.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Description of all changes.
    pub fn description(&self) -> &[String] {
        self.description.as_slice()
    }

    /// Adds to the value.
    pub fn add<V, S>(&mut self, rhs: V, reason: S)
    where
        V: Clone + Display,
        S: Display,
        T: Add<V, Output = T>,
    {
        let mut val = T::default();
        std::mem::swap(&mut val, &mut self.value);
        self.value = val.add(rhs.clone());
        self.description.push(format!("+{rhs} - {reason}"));
    }

    /// Subtracts from the value.
    pub fn sub<V, S>(&mut self, rhs: V, reason: S)
    where
        V: Clone + Display,
        S: Display,
        T: Sub<V, Output = T>,
    {
        let mut val = T::default();
        std::mem::swap(&mut val, &mut self.value);
        self.value = val.sub(rhs.clone());
        self.description.push(format!("-{rhs} - {reason}"));
    }

    /// Multiples the value.
    pub fn mul<V, S>(&mut self, rhs: V, reason: S)
    where
        V: Clone + Display,
        S: Display,
        T: Mul<V, Output = T>,
    {
        let mut val = T::default();
        std::mem::swap(&mut val, &mut self.value);
        self.value = val.mul(rhs.clone());
        self.description.push(format!("x{rhs} - {reason}"));
    }

    /// Divides the value.
    pub fn div<V, S>(&mut self, rhs: V, reason: S)
    where
        V: Clone + Display,
        S: Display,
        T: Div<V, Output = T>,
    {
        let mut val = T::default();
        std::mem::swap(&mut val, &mut self.value);
        self.value = val.div(rhs.clone());
        self.description.push(format!("\u{00F7}{rhs} - {reason}"));
    }

    /// Sets the value.
    pub fn set<V, S>(&mut self, rhs: V, reason: S)
    where
        V: Clone + Display + Into<T>,
        S: Display,
    {
        self.value = rhs.clone().into();
        self.description.push(format!("={rhs} - {reason}"));
    }

    /// Modifies the value.
    pub fn modify<F, S>(&mut self, f: F, reason: S)
    where
        F: FnOnce(&mut T),
        S: Display,
    {
        f(&mut self.value);
        self.description.push(format!("[modified] - {reason}"));
    }

    /// Maps to a value of another type.
    pub fn map<F, M, S>(mut self, f: F, reason: S) -> Output<M>
    where
        F: FnOnce(T) -> M,
        S: Display,
    {
        let value = f(self.value);
        self.description.push(format!("[mapped] - {reason}"));
        Output {
            value,
            description: self.description,
        }
    }
}

impl<T> From<T> for Output<T> {
    fn from(value: T) -> Self {
        Self {
            value,
            description: Vec::default(),
        }
    }
}

#[cfg(test)]
mod output_test {
    use battler_data::Fraction;

    use crate::common::Output;

    #[test]
    fn performs_arithmetic() {
        let mut output = Output::<Fraction<u64>>::default();

        output.add(2, "a");
        assert_eq!(output.value(), &2);
        assert_eq!(output.description().join(";"), "+2 - a");

        output.mul(1000, "b");
        assert_eq!(output.value(), &2000);
        assert_eq!(output.description().join(";"), "+2 - a;x1000 - b");

        output.sub(10, "c");
        assert_eq!(output.value(), &1990);
        assert_eq!(output.description().join(";"), "+2 - a;x1000 - b;-10 - c");

        output.div(3, "d");
        assert_eq!(output.value(), &Fraction::new(1990, 3));
        assert_eq!(
            output.description().join(";"),
            "+2 - a;x1000 - b;-10 - c;\u{00F7}3 - d"
        );

        output.modify(|val| *val = val.inverse(), "invert");
        assert_eq!(output.value(), &Fraction::new(3, 1990));
        assert_eq!(
            output.description().join(";"),
            "+2 - a;x1000 - b;-10 - c;\u{00F7}3 - d;[modified] - invert"
        );
    }
}
