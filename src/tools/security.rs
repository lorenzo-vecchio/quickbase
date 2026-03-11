use rand::prelude::*;
use rand::distr::Alphanumeric;
use rand_core::OsRng;
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use serde::{Serialize, Deserialize};

const ID_LENGTH: usize = 15;

/// Generates a random 15-character alphanumeric ID.
/// Mirrors PocketBase's security.RandomString(15).
pub fn generate_id() -> String {
    let mut rng = rand::rng();
    (0..ID_LENGTH)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect()
}

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

const JWT_SECRET: &[u8] = b"changeme_secret_key"; // will make configurable later

#[derive(Serialize, Deserialize)]
pub struct AuthClaims {
    pub sub: String,        // record id
    pub collection: String, // collection name
    pub exp: usize,         // expiry timestamp
}

pub fn generate_auth_token(record_id: &str, collection: &str) -> Result<String, jsonwebtoken::errors::Error> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize + 7 * 24 * 3600; // 7 days

    let claims = AuthClaims {
        sub: record_id.to_string(),
        collection: collection.to_string(),
        exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
}

pub fn verify_auth_token(token: &str) -> Result<AuthClaims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<AuthClaims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET),
        &validation,
    )
        .map(|data| data.claims)
}