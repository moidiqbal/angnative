use angnative_host_core::binary::{
    decode_binary_batch, encode_binary_batch, BinaryDecodeError, BINARY_HEADER_SIZE, BINARY_VERSION,
};
use angnative_host_core::command::{Command, CommandBatch};

const GOLDEN_HEX: &str = include_str!("../../../packages/testing/fixtures/binary-batch.hex");

fn hex_bytes() -> Vec<u8> {
    let hex = GOLDEN_HEX.trim();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn golden_batch() -> CommandBatch {
    let mut batch = CommandBatch::new(1, 1000);
    batch.push(Command::CreateNode {
        id: 1,
        node_type: "View".to_string(),
    });
    batch.push(Command::CreateNode {
        id: 2,
        node_type: "Text".to_string(),
    });
    batch.push(Command::InsertChild {
        parent_id: 1,
        child_id: 2,
        index: 0,
    });
    batch.push(Command::SetTextContent {
        id: 2,
        text: "hello ☃".to_string(),
    });
    batch.push(Command::SetStyle {
        id: 1,
        key: "flexDirection".to_string(),
        value: "row".to_string(),
    });
    batch.push(Command::SetProperty {
        id: 1,
        key: "disabled".to_string(),
        value: serde_json::json!(true),
    });
    batch.push(Command::RemoveChild {
        parent_id: 1,
        child_id: 2,
    });
    batch.push(Command::RemoveNode { id: 2 });
    batch
}

#[test]
fn decodes_shared_golden_vector() {
    let batch = decode_binary_batch(&hex_bytes()).expect("golden vector must decode");
    assert_eq!(batch.frame_id, 1);
    assert_eq!(batch.timestamp_ms, 1000);
    assert_eq!(batch.commands, golden_batch().commands);
}

#[test]
fn encodes_shared_golden_vector_byte_for_byte() {
    let hex: String = encode_binary_batch(&golden_batch())
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(hex, GOLDEN_HEX.trim());
}

#[test]
fn round_trips_all_command_kinds() {
    let bytes = encode_binary_batch(&golden_batch()).unwrap();
    let decoded = decode_binary_batch(&bytes).unwrap();
    assert_eq!(decoded.commands, golden_batch().commands);
    assert_eq!(BINARY_HEADER_SIZE, 26);
}

#[test]
fn rejects_bad_magic_and_unsupported_version() {
    let mut bytes = hex_bytes();
    bytes[0] = b'X';
    assert!(matches!(
        decode_binary_batch(&bytes),
        Err(BinaryDecodeError::BadMagic { .. })
    ));

    let mut bytes = hex_bytes();
    bytes[4..6].copy_from_slice(&(BINARY_VERSION + 1).to_le_bytes());
    assert!(matches!(
        decode_binary_batch(&bytes),
        Err(BinaryDecodeError::BadVersion { .. })
    ));
}

#[test]
fn rejects_truncated_frames_and_trailing_bytes() {
    let bytes = hex_bytes();
    assert!(matches!(
        decode_binary_batch(&bytes[..bytes.len() - 1]),
        Err(BinaryDecodeError::Truncated { .. })
    ));

    let mut trailing = bytes;
    trailing.push(0);
    assert!(matches!(
        decode_binary_batch(&trailing),
        Err(BinaryDecodeError::TrailingBytes { .. })
    ));
}

#[test]
fn rejects_unknown_opcode() {
    let mut bytes = hex_bytes();
    bytes[BINARY_HEADER_SIZE] = 0xff;
    assert!(matches!(
        decode_binary_batch(&bytes),
        Err(BinaryDecodeError::UnknownOpcode { opcode: 0xff, .. })
    ));
}

#[test]
fn rejects_invalid_utf8() {
    let mut bytes = hex_bytes();
    // First command: opcode at 26, id at 27..35, length prefix at 35..39.
    bytes[39] = 0xff;
    assert!(matches!(
        decode_binary_batch(&bytes),
        Err(BinaryDecodeError::InvalidUtf8 { .. })
    ));
}

#[test]
fn rejects_invalid_json_property_value() {
    let mut batch = CommandBatch::new(9, 9);
    batch.push(Command::SetProperty {
        id: 1,
        key: "k".to_string(),
        value: serde_json::json!("valid"),
    });
    let mut bytes = encode_binary_batch(&batch).unwrap();
    // Locate the final "valid" payload and break one byte into invalid JSON.
    let pos = bytes
        .windows(7)
        .position(|w| w == b"\"valid\"")
        .expect("value payload present");
    bytes[pos] = b'x';
    assert!(matches!(
        decode_binary_batch(&bytes),
        Err(BinaryDecodeError::InvalidJsonValue { .. })
    ));
}

#[test]
fn decoder_output_is_owned_and_survives_the_input_buffer() {
    let decoded = {
        let bytes = encode_binary_batch(&golden_batch()).unwrap();
        decode_binary_batch(&bytes).unwrap()
    };
    assert_eq!(decoded.commands, golden_batch().commands);
}

#[test]
fn refuses_unsupported_protocol_version_on_encode() {
    let mut batch = golden_batch();
    batch.protocol_version = 99;
    assert!(encode_binary_batch(&batch).is_err());
}
