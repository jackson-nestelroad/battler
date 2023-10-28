/// Helper trait for documenting that a lifetime is captured by a return type.
pub trait Captures<'a> {}
impl<'a, T: ?Sized> Captures<'a> for T {}
