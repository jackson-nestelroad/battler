use std::fmt;

use ahash::{
    HashMap,
    HashMapExt,
};
use serde::{
    Deserialize,
    Serialize,
    de::Visitor,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// The type of a species, which determines its weaknesses and resistances.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    SerializeLabeledStringEnum,
    DeserializeLabeledStringEnum,
)]
pub enum Type {
    #[string = "Normal"]
    #[default]
    Normal,
    #[string = "Fighting"]
    Fighting,
    #[string = "Flying"]
    Flying,
    #[string = "Poison"]
    Poison,
    #[string = "Ground"]
    Ground,
    #[string = "Rock"]
    Rock,
    #[string = "Bug"]
    Bug,
    #[string = "Ghost"]
    Ghost,
    #[string = "Steel"]
    Steel,
    #[string = "Fire"]
    Fire,
    #[string = "Water"]
    Water,
    #[string = "Grass"]
    Grass,
    #[string = "Electric"]
    Electric,
    #[string = "Psychic"]
    Psychic,
    #[string = "Ice"]
    Ice,
    #[string = "Dragon"]
    Dragon,
    #[string = "Dark"]
    Dark,
    #[string = "Fairy"]
    Fairy,
    #[string = "None"]
    None,
    #[string = "Stellar"]
    Stellar,
}

/// Type effectiveness of one type against another.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum TypeEffectiveness {
    /// No effect.
    None,
    /// Not very effective.
    Weak,
    /// Normal effectiveness.
    #[default]
    Normal,
    /// Super effective.
    Strong,
}

impl From<f32> for TypeEffectiveness {
    fn from(value: f32) -> Self {
        if value < 0f32 || (value).abs() < f32::EPSILON {
            Self::None
        } else if value < 0.5 || (value - 0.5).abs() < f32::EPSILON {
            Self::Weak
        } else if value < 1f32 || (value - 1f32).abs() < f32::EPSILON {
            Self::Normal
        } else {
            Self::Strong
        }
    }
}

impl Into<f32> for TypeEffectiveness {
    fn into(self) -> f32 {
        match self {
            Self::None => 0f32,
            Self::Weak => 0.5,
            Self::Normal => 1f32,
            Self::Strong => 2f32,
        }
    }
}

impl Serialize for TypeEffectiveness {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self {
            Self::None => serializer.serialize_u32(Into::<f32>::into(*self) as u32),
            TypeEffectiveness::Weak => serializer.serialize_f32(Into::<f32>::into(*self)),
            Self::Normal => serializer.serialize_u32(Into::<f32>::into(*self) as u32),
            Self::Strong => serializer.serialize_u32(Into::<f32>::into(*self) as u32),
        }
    }
}

struct TypeEffectivenessVisitor;

impl<'de> Visitor<'de> for TypeEffectivenessVisitor {
    type Value = TypeEffectiveness;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("one of the following values: 0, 0.5, 1, 2")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v as f32))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v as f32))
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v as f32))
    }
}

impl<'de> Deserialize<'de> for TypeEffectiveness {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_f32(TypeEffectivenessVisitor)
    }
}

/// A type table, which contains type effectiveness information for types against some other value.
///
/// The key here is the attacking type.
pub type TypeTable<T> = HashMap<Type, HashMap<T, TypeEffectiveness>>;

/// A type chart, which contains all type effectiveness information for types against other types
/// and effects.
///
/// The key here is the attacking type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeChart {
    pub types: TypeTable<Type>,
}

impl TypeChart {
    pub fn new() -> Self {
        Self {
            types: TypeTable::new(),
        }
    }

    pub fn from_filled(types: TypeTable<Type>) -> Self {
        Self { types }
    }
}

#[cfg(test)]
mod type_test {
    use crate::{
        Type,
        test_util::{
            test_string_deserialization,
            test_string_serialization,
        },
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(Type::Grass, "Grass");
        test_string_serialization(Type::Fire, "Fire");
        test_string_serialization(Type::Water, "Water");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("normal", Type::Normal);
        test_string_deserialization("dragon", Type::Dragon);
        test_string_deserialization("ghost", Type::Ghost);
    }
}

#[cfg(test)]
mod type_effectiveness_test {
    use ahash::HashMap;

    use crate::{
        Type,
        TypeChart,
        TypeEffectiveness,
        TypeTable,
        test_util::test_deserialization,
    };

    #[test]
    fn serializes_to_number() {
        test_deserialization("0", TypeEffectiveness::None);
        test_deserialization("0.5", TypeEffectiveness::Weak);
        test_deserialization("1", TypeEffectiveness::Normal);
        test_deserialization("2", TypeEffectiveness::Strong);
    }

    #[test]
    fn deserializes_type_table() {
        let str = r#"{
            "Fire": {
                "Fire": 0.5,
                "Water": 0.5,
                "Grass": 2,
                "Ice": 2,
                "Bug": 2,
                "Rock": 0.5,
                "Dragon": 0.5,
                "Steel": 2
            }
        }"#;
        let tc = serde_json::from_str::<TypeTable<Type>>(str).unwrap();
        let expected = TypeTable::from_iter([(
            Type::Fire,
            HashMap::from_iter([
                (Type::Fire, TypeEffectiveness::Weak),
                (Type::Water, TypeEffectiveness::Weak),
                (Type::Grass, TypeEffectiveness::Strong),
                (Type::Ice, TypeEffectiveness::Strong),
                (Type::Bug, TypeEffectiveness::Strong),
                (Type::Rock, TypeEffectiveness::Weak),
                (Type::Dragon, TypeEffectiveness::Weak),
                (Type::Steel, TypeEffectiveness::Strong),
            ]),
        )]);
        assert_eq!(tc, expected)
    }

    #[test]
    fn deserializes_type_chart() {
        let str = r#"{
           "types": {
                "Fire": {
                    "Fire": 0.5,
                    "Water": 0.5,
                    "Grass": 2,
                    "Ice": 2,
                    "Bug": 2,
                    "Rock": 0.5,
                    "Dragon": 0.5,
                    "Steel": 2
                }
            }
        }"#;
        let tc = serde_json::from_str::<TypeChart>(str).unwrap();
        let expected = TypeChart::from_filled(TypeTable::from_iter([(
            Type::Fire,
            HashMap::from_iter([
                (Type::Fire, TypeEffectiveness::Weak),
                (Type::Water, TypeEffectiveness::Weak),
                (Type::Grass, TypeEffectiveness::Strong),
                (Type::Ice, TypeEffectiveness::Strong),
                (Type::Bug, TypeEffectiveness::Strong),
                (Type::Rock, TypeEffectiveness::Weak),
                (Type::Dragon, TypeEffectiveness::Weak),
                (Type::Steel, TypeEffectiveness::Strong),
            ]),
        )]));
        assert_eq!(tc, expected)
    }
}
