/// How a procedure registration or subscription should be matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchStyle {
    Prefix,
    Wildcard,
}

impl TryFrom<&str> for MatchStyle {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "prefix" => Ok(Self::Prefix),
            "wildcard" => Ok(Self::Wildcard),
            _ => Err(Self::Error::msg(format!("invalid match style: {value}"))),
        }
    }
}

impl Into<&'static str> for MatchStyle {
    fn into(self) -> &'static str {
        match self {
            Self::Prefix => "prefix",
            Self::Wildcard => "wildcard",
        }
    }
}

impl Into<String> for MatchStyle {
    fn into(self) -> String {
        Into::<&'static str>::into(self).to_owned()
    }
}

impl ToString for MatchStyle {
    fn to_string(&self) -> String {
        (*self).into()
    }
}
