use core::{
    fmt,
    ops::{
        Deref,
        DerefMut,
        Not,
    },
};

/// A boolean type that defaults to `true`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefaultTrueBool(pub bool);

impl Default for DefaultTrueBool {
    fn default() -> Self {
        DefaultTrueBool(true)
    }
}

impl Deref for DefaultTrueBool {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DefaultTrueBool {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<bool> for DefaultTrueBool {
    fn from(b: bool) -> Self {
        DefaultTrueBool(b)
    }
}

impl From<DefaultTrueBool> for bool {
    fn from(b: DefaultTrueBool) -> Self {
        b.0
    }
}

impl fmt::Display for DefaultTrueBool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Not for DefaultTrueBool {
    type Output = Self;

    fn not(self) -> Self::Output {
        DefaultTrueBool(!self.0)
    }
}
