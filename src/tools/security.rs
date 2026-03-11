use rand::prelude::*;
use rand::distr::Alphanumeric;

const ID_LENGTH: usize = 15;

/// Generates a random 15-character alphanumeric ID.
/// Mirrors PocketBase's security.RandomString(15).
pub fn generate_id() -> String {
    let mut rng = rand::rng();
    (0..ID_LENGTH)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect()
}