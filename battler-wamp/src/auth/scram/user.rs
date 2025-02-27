use anyhow::Result;
use async_trait::async_trait;
use password_hash::{
    SaltString,
    rand_core::OsRng,
};

use crate::auth::{
    identity::Identity,
    key_derivation_function::KeyDerivationFunction,
    scram::core::{
        client_key,
        salt_password,
        server_key,
        stored_key,
    },
};

/// Per-user data required for WAMP-SCRAM.
#[derive(Clone)]
pub struct UserData {
    /// User identity.
    pub identity: Identity,
    /// Random, per-user salt.
    pub salt: SaltString,
    /// The key derivation function used to has the password.
    pub key_derivation_function: KeyDerivationFunction,
    /// Number of iterations.
    pub iterations: u32,
    /// Amount of memory.
    pub memory: Option<u32>,
    /// The stored key.
    pub stored_key: [u8; 32],
    /// The server key.
    pub server_key: Vec<u8>,
}

/// Database for looking up [`UserData`] for a user.
#[async_trait]
pub trait UserDatabase: Send + Sync {
    /// Looks up per-user data for WAMP-SCRAM.
    async fn user_data(&self, id: &str) -> Result<UserData>;
}

/// Generates a new user for the given password.
pub fn new_user(id: &str, password: &str) -> Result<UserData> {
    let key_derivation_function = KeyDerivationFunction::Argon2Id13;
    let iterations = key_derivation_function.recommended_iterations();
    let memory = key_derivation_function.recommended_memory();

    let password = stringprep::saslprep(password)?;

    let salt = SaltString::generate(&mut OsRng);
    let salted_password = salt_password(
        &password,
        salt.as_salt(),
        key_derivation_function,
        iterations,
        memory,
    )?;

    let client_key = client_key(salted_password.as_bytes())?;
    let stored_key = stored_key(&client_key)?;
    let server_key = server_key(salted_password.as_bytes())?;

    Ok(UserData {
        identity: Identity {
            id: id.to_owned(),
            role: "user".to_owned(),
        },
        salt,
        key_derivation_function,
        iterations,
        memory,
        stored_key,
        server_key,
    })
}
