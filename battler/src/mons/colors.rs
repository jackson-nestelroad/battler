use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// A defined enumeration of species colors.
#[derive(Debug, Clone, PartialEq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
pub enum Color {
    #[string = "Red"]
    Red,
    #[string = "Blue"]
    Blue,
    #[string = "Yellow"]
    Yellow,
    #[string = "Green"]
    Green,
    #[string = "Black"]
    Black,
    #[string = "Brown"]
    Brown,
    #[string = "Purple"]
    Purple,
    #[string = "Gray"]
    Gray,
    #[string = "White"]
    White,
    #[string = "Pink"]
    Pink,
}

#[cfg(test)]
mod color_tests {
    use crate::{
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
        mons::Color,
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(Color::Red, "Red");
        test_string_serialization(Color::Blue, "Blue");
        test_string_serialization(Color::Yellow, "Yellow");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("green", Color::Green);
        test_string_deserialization("black", Color::Black);
        test_string_deserialization("brown", Color::Brown);
    }
}
