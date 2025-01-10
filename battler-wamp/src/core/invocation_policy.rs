/// How a callee should be selected for invocations.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InvocationPolicy {
    /// Invocation is sent to a single callee.
    #[default]
    Single,
    /// Invocation is sent to a callee in order of registration.
    RoundRobin,
    /// Invocation is sent to a random callee.
    Random,
    /// Invocation is sent to the first callee.
    First,
    /// Invocation is sent to the last callee.
    Last,
}

impl TryFrom<&str> for InvocationPolicy {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "single" => Ok(Self::Single),
            "roundrobin" => Ok(Self::RoundRobin),
            "random" => Ok(Self::Random),
            "first" => Ok(Self::First),
            "last" => Ok(Self::Last),
            _ => Err(Self::Error::msg(format!(
                "invalid invocation policy: {value}"
            ))),
        }
    }
}

impl Into<&'static str> for InvocationPolicy {
    fn into(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::RoundRobin => "roundrobin",
            Self::Random => "random",
            Self::First => "first",
            Self::Last => "last",
        }
    }
}

impl Into<String> for InvocationPolicy {
    fn into(self) -> String {
        Into::<&'static str>::into(self).to_owned()
    }
}

impl ToString for InvocationPolicy {
    fn to_string(&self) -> String {
        (*self).into()
    }
}
