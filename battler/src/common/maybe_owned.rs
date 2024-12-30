use std::{
    fmt,
    fmt::{
        Debug,
        Display,
    },
    hash,
    hash::Hash,
    ops::{
        Deref,
        DerefMut,
    },
};

/// A value that may or may not be owned.
pub enum MaybeOwned<'a, T> {
    Owned(T),
    Unowned(&'a T),
}

/// A mutable value that may or may not be owned.
pub enum MaybeOwnedMut<'a, T> {
    Owned(T),
    Unowned(&'a mut T),
}

macro_rules! common_maybe_owned_impls {
    ($name:ident) => {
        impl<T> $name<'_, T>
        where
            T: Clone,
        {
            /// Clones the maybe-owned object into an owned `T` value.
            pub fn clone_owned(&self) -> T {
                match self {
                    Self::Owned(value) => value.clone(),
                    Self::Unowned(value) => value.deref().clone(),
                }
            }
        }

        impl<T> From<T> for $name<'_, T> {
            fn from(value: T) -> Self {
                Self::Owned(value)
            }
        }

        impl<T> AsRef<T> for $name<'_, T> {
            fn as_ref(&self) -> &T {
                match self {
                    Self::Owned(value) => &value,
                    Self::Unowned(value) => value,
                }
            }
        }

        impl<T> Deref for $name<'_, T> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                self.as_ref()
            }
        }

        impl<T> PartialEq for $name<'_, T>
        where
            T: PartialEq,
        {
            fn eq(&self, other: &Self) -> bool {
                PartialEq::eq(self.as_ref(), other.as_ref())
            }
        }

        impl<T> Eq for $name<'_, T> where T: PartialEq {}

        impl<T> Hash for $name<'_, T>
        where
            T: Hash,
        {
            fn hash<H: hash::Hasher>(&self, state: &mut H) {
                Hash::hash(self.as_ref(), state)
            }
        }

        impl<T> Display for $name<'_, T>
        where
            T: Display,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.as_ref())
            }
        }

        impl<T> Debug for $name<'_, T>
        where
            T: Debug,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                Debug::fmt(self.as_ref(), f)
            }
        }
    };
}

common_maybe_owned_impls!(MaybeOwned);
common_maybe_owned_impls!(MaybeOwnedMut);

impl<'a, T> MaybeOwned<'a, T> {
    pub fn unowned_ref<'b>(&'a self) -> MaybeOwned<'b, T>
    where
        'a: 'b,
    {
        match self {
            Self::Owned(value) => Self::Unowned(&value),
            Self::Unowned(value) => Self::Unowned(value),
        }
    }
}

impl<'a, T> From<&'a T> for MaybeOwned<'a, T> {
    fn from(value: &'a T) -> Self {
        Self::Unowned(value)
    }
}

impl<'a, T> From<&'a mut T> for MaybeOwnedMut<'a, T> {
    fn from(value: &'a mut T) -> Self {
        Self::Unowned(value)
    }
}

impl<T> Clone for MaybeOwned<'_, T>
where
    T: Clone + ?Sized,
{
    fn clone(&self) -> Self {
        match self {
            Self::Owned(value) => Self::Owned(value.clone()),
            Self::Unowned(value) => Self::Unowned(value),
        }
    }
}

impl<T> AsMut<T> for MaybeOwnedMut<'_, T> {
    fn as_mut(&mut self) -> &mut T {
        match self {
            Self::Owned(value) => value,
            Self::Unowned(value) => value,
        }
    }
}

impl<T> DerefMut for MaybeOwnedMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Owned(value) => value,
            Self::Unowned(value) => value,
        }
    }
}
