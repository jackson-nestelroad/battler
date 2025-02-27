use anyhow::{
    Error,
    Result,
};
use argon2::PasswordHasher;
use battler_wamp_values::{
    Value,
    WampDeserialize,
    WampDeserializeError,
    WampSerialize,
    WampSerializeError,
};
use password_hash::{
    PasswordHashString,
    Salt,
};

/// Password-based key derivation function (KDF) to hash user passwords.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDerivationFunction {
    /// Argon2id variant of Argon2, version 1.3.
    Argon2Id13,
    /// PBKDF2.
    Pbkdf2,
}

impl KeyDerivationFunction {
    /// Recommended number of iterations.
    pub fn recommended_iterations(&self) -> u32 {
        match self {
            Self::Argon2Id13 => argon2::Params::DEFAULT_T_COST,
            Self::Pbkdf2 => pbkdf2::Params::RECOMMENDED_ROUNDS as u32,
        }
    }

    /// Recommended memory.
    pub fn recommended_memory(&self) -> Option<u32> {
        match self {
            Self::Argon2Id13 => Some(argon2::Params::DEFAULT_M_COST),
            Self::Pbkdf2 => None,
        }
    }

    /// Runs the key derivation function.
    pub fn run(
        &self,
        password: &str,
        salt: Salt,
        iterations: u32,
        memory: Option<u32>,
    ) -> Result<PasswordHashString> {
        match self {
            Self::Argon2Id13 => {
                let params = argon2::Params::new(
                    memory.unwrap_or(argon2::Params::DEFAULT_M_COST),
                    iterations,
                    argon2::Params::DEFAULT_P_COST,
                    None,
                )
                .map_err(|err| Error::msg(format!("failed to create argon2 params: {err:?}")))?;
                let argon2 = argon2::Argon2::new(
                    argon2::Algorithm::Argon2id,
                    argon2::Version::V0x13,
                    params,
                );
                let hash = argon2
                    .hash_password(password.as_bytes(), salt)
                    .map_err(|err| Error::msg(format!("failed to hash password: {err:?}")))?;
                Ok(hash.serialize())
            }
            Self::Pbkdf2 => {
                let mut params = pbkdf2::Params::default();
                params.rounds = iterations;
                let hash = pbkdf2::Pbkdf2
                    .hash_password_customized(password.as_bytes(), None, None, params, salt)
                    .map_err(|err| Error::msg(format!("failed to hash password: {err:?}")))?;
                Ok(hash.serialize())
            }
        }
    }
}

impl TryFrom<&str> for KeyDerivationFunction {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "argon2id13" => Ok(Self::Argon2Id13),
            "pbkdf2" => Ok(Self::Pbkdf2),
            _ => Err(Self::Error::msg(format!(
                "invalid key derivation function: {value}"
            ))),
        }
    }
}

impl Into<&'static str> for KeyDerivationFunction {
    fn into(self) -> &'static str {
        match self {
            Self::Argon2Id13 => "argon2id13",
            Self::Pbkdf2 => "pbkdf2",
        }
    }
}

impl Into<String> for KeyDerivationFunction {
    fn into(self) -> String {
        Into::<&'static str>::into(self).to_owned()
    }
}

impl WampSerialize for KeyDerivationFunction {
    fn wamp_serialize(self) -> Result<Value, WampSerializeError> {
        Ok(Value::String(self.into()))
    }
}

impl WampDeserialize for KeyDerivationFunction {
    fn wamp_deserialize(value: Value) -> Result<Self, WampDeserializeError> {
        value
            .string()
            .ok_or_else(|| WampDeserializeError::new("key derivation function must be a string"))?
            .try_into()
            .map_err(|err: anyhow::Error| WampDeserializeError::new(err.to_string()))
    }
}
