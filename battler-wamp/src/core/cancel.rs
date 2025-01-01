/// How an active procedure call should be canceled.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CallCancelMode {
    /// An ERROR is sent immediately back to the caller. The callee receives no INTERRUPT.
    #[default]
    Skip,
    /// INTERRUPT is sent to the callee, and the caller waits for acknowledgement in the form of an
    /// ERROR or RESULT.
    Kill,
    /// INTERRUPT is sent to the callee, and an ERROR is sent immediately back to the caller.
    KillNoWait,
}

impl TryFrom<&str> for CallCancelMode {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "skip" => Ok(Self::Skip),
            "kill" => Ok(Self::Kill),
            "killnowait" => Ok(Self::KillNoWait),
            _ => Err(Self::Error::msg(format!(
                "invalid call cancel mode: {value}"
            ))),
        }
    }
}

impl Into<&'static str> for CallCancelMode {
    fn into(self) -> &'static str {
        match self {
            Self::Skip => "skip",
            Self::Kill => "kill",
            Self::KillNoWait => "killnowait",
        }
    }
}

impl Into<String> for CallCancelMode {
    fn into(self) -> String {
        Into::<&'static str>::into(self).to_owned()
    }
}

impl ToString for CallCancelMode {
    fn to_string(&self) -> String {
        (*self).into()
    }
}
