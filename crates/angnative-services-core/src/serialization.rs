/**
 * serialization.rs — Serialization helpers for angnative-services-core.
 *
 * Provides JSON serialization with consistent error types. Used by the
 * command protocol, offline-sync queue, and diagnostics channel.
 */

use serde::{de::DeserializeOwned, Serialize};

/** Error returned when serialization fails. */
#[derive(Debug, thiserror::Error)]
pub enum SerializeError {
    #[error("JSON serialization failed: {0}")]
    Json(String),
}

/** Error returned when deserialization fails. */
#[derive(Debug, thiserror::Error)]
pub enum DeserializeError {
    #[error("JSON deserialization failed: {0}")]
    Json(String),
}

/// Serialization format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerializationFormat {
    Json,
}

/// Serialize a value to JSON bytes.
pub fn serialize_json<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, SerializeError> {
    serde_json::to_vec(value).map_err(|e| SerializeError::Json(e.to_string()))
}

/// Deserialize a value from JSON bytes.
pub fn deserialize_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, DeserializeError> {
    serde_json::from_slice(bytes).map_err(|e| DeserializeError::Json(e.to_string()))
}

/// Serialize a value to the given format (currently only JSON supported).
pub fn serialize<T: Serialize + ?Sized>(
    value: &T,
    _format: SerializationFormat,
) -> Result<Vec<u8>, SerializeError> {
    serialize_json(value)
}

/// Deserialize a value from the given format (currently only JSON supported).
pub fn deserialize<T: DeserializeOwned>(
    bytes: &[u8],
    _format: SerializationFormat,
) -> Result<T, DeserializeError> {
    deserialize_json(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct TestValue {
        name: String,
        value: i32,
        nested: Option<Vec<String>>,
    }

    #[test]
    fn serialize_deserialize_json_round_trip() {
        let original = TestValue {
            name: "test".to_string(),
            value: 42,
            nested: Some(vec!["a".to_string(), "b".to_string()]),
        };

        let bytes = serialize_json(&original).unwrap();
        let deserialized: TestValue = deserialize_json(&bytes).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn serialize_json_error_type() {
        let err = SerializeError::Json("invalid data".to_string());
        assert!(err.to_string().contains("invalid data"));

        let err = DeserializeError::Json("bad json".to_string());
        assert!(err.to_string().contains("bad json"));
    }

    #[test]
    fn serialization_format_enum() {
        let json = SerializationFormat::Json;
        assert!(matches!(json, SerializationFormat::Json));
    }

    #[test]
    fn serialize_generic_helper() {
        let original = TestValue {
            name: "generic".to_string(),
            value: 99,
            nested: None,
        };

        let bytes = serialize(&original, SerializationFormat::Json).unwrap();
        let deserialized: TestValue = deserialize(&bytes, SerializationFormat::Json).unwrap();
        assert_eq!(original, deserialized);
    }
}
