use serde_json::Value;

pub fn get_value_from_jsep(event: &Value, key: &str) -> Option<String> {
    let val = event
        .get("event")
        .and_then(|e| e.get("jsep"))
        .and_then(|j| j.get(key))
        .and_then(|t| t.as_str());
    val.map(|s| s.to_string())
}
