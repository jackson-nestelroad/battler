use anyhow::{
    Error,
    Result,
};
use base64::Engine;
use hmac::Mac;
use password_hash::{
    PasswordHashString,
    Salt,
};
use sha2::Digest;

use crate::auth::{
    channel_binding::ChannelBinding,
    key_derivation_function::KeyDerivationFunction,
};

/// Salts a user password.
pub fn salt_password(
    password: &str,
    salt: Salt,
    key_derivation_function: KeyDerivationFunction,
    iterations: u32,
    memory: Option<u32>,
) -> Result<PasswordHashString> {
    key_derivation_function.run(&password, salt, iterations, memory)
}

/// Client key.
pub fn client_key(salted_password: &[u8]) -> Result<Vec<u8>> {
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(salted_password)?;
    mac.update(b"Client Key");
    Ok(mac.finalize().into_bytes().to_vec())
}

/// Stored client key.
pub fn stored_key(client_key: &[u8]) -> Result<[u8; 32]> {
    sha2::Sha256::digest(client_key)
        .try_into()
        .map_err(Error::new)
}

/// Server key.
pub fn server_key(salted_password: &[u8]) -> Result<Vec<u8>> {
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(salted_password)?;
    mac.update(b"Server Key");
    Ok(mac.finalize().into_bytes().to_vec())
}

fn escape(s: &str) -> String {
    s.replace(',', "=2C").replace('=', "=3D")
}

/// Authentication message used for computing the [`client_proof`] and [`server_signature`].
pub fn auth_message(
    id: &str,
    client_nonce: &str,
    server_nonce: &str,
    salt: &str,
    iterations: u32,
    channel_binding: Option<ChannelBinding>,
    cbind_data: Option<&str>,
) -> Result<String> {
    let id = escape(id);
    let client_first_bare = format!("n={id},r={client_nonce}");
    let server_first = format!("r={server_nonce},s={salt},i={iterations}");
    let cbind_flag = match channel_binding {
        Some(channel_binding) => format!("p={channel_binding}"),
        None => "n".to_owned(),
    };
    let cbind_input = format!(
        "{cbind_flag},,{}",
        match cbind_data {
            Some(cbind_data) => base64::prelude::BASE64_STANDARD
                .decode(cbind_data)?
                .into_iter()
                .map(|c| c as char)
                .collect(),
            None => "".to_owned(),
        }
    );
    let client_final_no_proof = format!(
        "c={},r={client_nonce}{server_nonce}",
        base64::prelude::BASE64_STANDARD.encode(cbind_input)
    );
    Ok(format!(
        "{client_first_bare},{server_first},{client_final_no_proof}"
    ))
}

/// Client signature.
pub fn client_signature(stored_key: &[u8], auth_message: &[u8]) -> Result<Vec<u8>> {
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(stored_key)?;
    mac.update(auth_message);
    Ok(mac.finalize().into_bytes().to_vec())
}

/// Client proof.
pub fn client_proof(client_key: &[u8], client_signature: &[u8]) -> Vec<u8> {
    client_key
        .iter()
        .zip(client_signature.iter())
        .map(|(a, b)| a ^ b)
        .collect()
}

/// Recovered client key.
pub fn recovered_client_key(client_signature: &[u8], received_client_proof: &[u8]) -> Vec<u8> {
    client_signature
        .iter()
        .zip(received_client_proof.iter())
        .map(|(a, b)| a ^ b)
        .collect()
}

/// Server signature.
pub fn server_signature(server_key: &[u8], auth_message: &[u8]) -> Result<Vec<u8>> {
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(server_key)?;
    mac.update(auth_message);
    Ok(mac.finalize().into_bytes().to_vec())
}
