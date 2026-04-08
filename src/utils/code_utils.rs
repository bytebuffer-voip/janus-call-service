use rand::distr::Alphanumeric;
use rand::prelude::IteratorRandom;
use rand::{rng, RngExt};

pub fn random_otp() -> String {
    let mut rng = rng();
    let random_string: String = (0..6)
        .map(|_| rng.random_range(0..10).to_string())
        .collect();
    random_string
}

pub fn generate_user_id() -> String {
    let random_string: String = rng()
        .sample_iter(&Alphanumeric)
        .take(20)
        .map(char::from)
        .collect();
    random_string
}

pub fn generate_id(n: usize) -> String {
    let random_string: String = rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .map(char::from)
        .collect();
    random_string
}

pub fn generate_strong_password(length: usize) -> String {
    let charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()-_=+";
    let mut rng = rng();
    (0..length)
        .map(|_| charset.chars().choose(&mut rng).unwrap())
        .collect()
}
