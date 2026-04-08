use log::info;
use regex::Regex;

pub fn is_valid_email(email: &str) -> bool {
    let email_regex =
        match Regex::new(r"(?i)^[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9.-]+\.[a-z]{2,}$") {
            Ok(r) => r,
            Err(e) => {
                info!("is_valid_email ex: {:?}", e);
                return false;
            }
        };
    email_regex.is_match(email)
}
