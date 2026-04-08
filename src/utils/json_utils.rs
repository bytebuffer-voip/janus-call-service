use mongodb::bson;
use mongodb::bson::Document;
use serde::Serialize;
use serde_json::Value;

pub fn json_str_to_bson_doc(json_str: &str) -> anyhow::Result<Document> {
    let v: Value = serde_json::from_str(json_str)?;
    let doc = bson::to_document(&v)?;
    Ok(doc)
}

pub fn to_string<T>(value: &T) -> anyhow::Result<String>
where
    T: ?Sized + Serialize,
{
    let json = serde_json::to_string(&value)?;
    Ok(json)
}

pub fn get_string_value<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(|v| v.as_str()).unwrap_or_default()
}

pub fn get_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(|v| v.as_str())
}

pub fn get_u32_value(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(|v| v.as_i64()).unwrap_or(-1)
}

pub fn get_value(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

pub fn get_int_value(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(|v| v.as_i64()).unwrap_or_default()
}
pub fn get_int(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(|v| v.as_i64())
}
