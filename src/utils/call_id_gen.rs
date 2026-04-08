use crate::utils::code_utils;

pub fn gen_call_id() -> String {
    let rand_string = code_utils::generate_id(8);
    let timestamp = chrono::Utc::now().timestamp_millis();
    format!("call-{}-{}", rand_string, timestamp).to_lowercase()
}
