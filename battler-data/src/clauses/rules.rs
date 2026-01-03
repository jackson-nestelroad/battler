use alloc::{
    borrow::ToOwned,
    string::String,
};
use core::{
    fmt,
    fmt::Display,
    hash,
    hash::Hash,
    str::FromStr,
};

use anyhow::{
    Error,
    Result,
};
use hashbrown::HashSet;
use serde_string_enum::{
    DeserializeStringEnum,
    SerializeStringEnum,
};

use crate::Id;

/// A single rule that must be validated before a battle starts.
#[derive(Debug, Clone, Eq, SerializeStringEnum, DeserializeStringEnum)]
pub enum Rule {
    /// Bans something, such as a Mon, item, move, or ability. Serialized as `- ID`.
    Ban(Id),
    /// Unbans something, such as a Mon, item, move, or ability. Serialized as `+ ID`.
    ///
    /// An unban is used to override a ban rule that is typically more general. For example, `-
    /// Legendary, + Giratina` would allow the Mon `Giratina` to be used, even though it is a
    /// legendary.
    Unban(Id),
    /// Some other rule attached to a value. Serialized as `name = value`.
    ///
    /// If `value` is empty, then the rule is simply serialized as `name`.
    Value { name: Id, value: String },
    /// Repeals a previously established rule. Serialized as `! name`.
    ///
    /// Compound and single rules can be repealed. Bans and unbans cannot be repealed.
    Repeal(Id),
}

impl Rule {
    /// Constructs a new named rule without a value.
    pub fn value_name(name: &str) -> Rule {
        Rule::Value {
            name: Id::from(name),
            value: String::new(),
        }
    }

    /// Constructs a new named rule without a value, directly from an ID.
    pub fn value_id(name: Id) -> Rule {
        Rule::Value {
            name: name,
            value: String::new(),
        }
    }
}

impl Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ban(id) => write!(f, "-{id}"),
            Self::Unban(id) => write!(f, "+{id}"),
            Self::Value { name, value } => {
                if value.is_empty() {
                    write!(f, "{name}")
                } else {
                    write!(f, "{name}={value}")
                }
            }
            Self::Repeal(id) => write!(f, "!{id}"),
        }
    }
}

impl FromStr for Rule {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &s[0..1] {
            "-" => Ok(Self::Ban(Id::from(s[1..].trim()))),
            "+" => Ok(Self::Unban(Id::from(s[1..].trim()))),
            "!" => Ok(Self::Repeal(Id::from(s[1..].trim()))),
            _ => match s.split_once('=') {
                None => Ok(Self::Value {
                    name: Id::from(s.trim()),
                    value: "".to_owned(),
                }),
                Some((name, value)) => Ok(Self::Value {
                    name: Id::from(name.trim()),
                    value: value.trim().to_owned(),
                }),
            },
        }
    }
}

impl Hash for Rule {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Ban(id) => id.hash(state),
            Self::Unban(id) => id.hash(state),
            Self::Value { name, .. } => name.hash(state),
            Self::Repeal(id) => id.hash(state),
        }
    }
}

impl PartialEq for Rule {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Ban(id) => match other {
                Self::Ban(other) => id.eq(other),
                _ => false,
            },
            Self::Unban(id) => match other {
                Self::Unban(other) => id.eq(other),
                _ => false,
            },
            Self::Value { name, value: _ } => match other {
                Self::Value {
                    name: other,
                    value: _,
                } => name.eq(other),
                _ => false,
            },
            Self::Repeal(id) => match other {
                Self::Repeal(other) => id.eq(other),
                _ => false,
            },
        }
    }
}

/// A user-defined set of rules.
pub type SerializedRuleSet = HashSet<Rule>;

#[cfg(test)]
mod rule_test {
    use alloc::borrow::ToOwned;

    use crate::{
        Id,
        Rule,
        test_util::test_string_serialization,
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(Rule::Ban(Id::from("bulbasaur")), "-bulbasaur");
        test_string_serialization(Rule::Ban(Id::from("Giratina (Origin)")), "-giratinaorigin");
        test_string_serialization(Rule::Unban(Id::from("Porygon-Z")), "+porygonz");
        test_string_serialization(Rule::Repeal(Id::from("Evasion Clause")), "!evasionclause");
        test_string_serialization(
            Rule::Value {
                name: Id::from("Max Level"),
                value: "50".to_owned(),
            },
            "maxlevel=50",
        );
    }
}
