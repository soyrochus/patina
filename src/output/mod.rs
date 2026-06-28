pub mod error;

pub use error::{ErrorEntry, WarningEntry};
use serde::Serialize;

pub const JSON_ENVELOPE_VERSION: &str = "0.1";

#[derive(Debug, Serialize)]
pub struct JsonEnvelope<T>
where
    T: Serialize,
{
    pub version: String,
    pub command: String,
    pub ok: bool,
    pub data: Option<T>,
    pub warnings: Vec<WarningEntry>,
    pub errors: Vec<ErrorEntry>,
}

impl<T> JsonEnvelope<T>
where
    T: Serialize,
{
    pub fn new(
        command: &str,
        ok: bool,
        data: Option<T>,
        warnings: Vec<WarningEntry>,
        errors: Vec<ErrorEntry>,
    ) -> Self {
        Self {
            version: JSON_ENVELOPE_VERSION.to_string(),
            command: command.to_string(),
            ok,
            data,
            warnings,
            errors,
        }
    }

    pub fn success(command: &str, data: T, warnings: Vec<WarningEntry>) -> Self {
        Self {
            version: JSON_ENVELOPE_VERSION.to_string(),
            command: command.to_string(),
            ok: true,
            data: Some(data),
            warnings,
            errors: Vec::new(),
        }
    }

    pub fn failure(command: &str, error: ErrorEntry) -> JsonEnvelope<serde_json::Value> {
        JsonEnvelope {
            version: JSON_ENVELOPE_VERSION.to_string(),
            command: command.to_string(),
            ok: false,
            data: None,
            warnings: Vec::new(),
            errors: vec![error],
        }
    }
}

pub fn print_json<T>(envelope: &JsonEnvelope<T>) -> anyhow::Result<()>
where
    T: Serialize,
{
    println!("{}", serde_json::to_string_pretty(envelope)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_all_envelope_fields() {
        let envelope = JsonEnvelope::success("lint", json!({"ok": true}), Vec::new());
        let value = serde_json::to_value(envelope).expect("envelope should serialize");

        for key in ["version", "command", "ok", "data", "warnings", "errors"] {
            assert!(value.get(key).is_some(), "missing key {key}");
        }
        assert!(value["warnings"].is_array());
        assert!(value["errors"].is_array());
    }
}
