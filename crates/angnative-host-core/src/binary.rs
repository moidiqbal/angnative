/**
 * binary.rs — versioned ANB1 binary mutation protocol (Rust decoder/encoder).
 *
 * Mirrors `packages/contract/src/binary.ts` byte-for-byte. Both formats decode
 * to the same CommandBatch model; JSON remains the fallback wire format.
 *
 * Framing (little-endian): magic "ANB1" | u16 version | u64 frameId
 * | u64 timestampMs | u32 commandCount | commands...
 */

use crate::command::{Command, CommandBatch, PROTOCOL_VERSION};
use thiserror::Error;

pub const BINARY_MAGIC: [u8; 4] = *b"ANB1";
pub const BINARY_VERSION: u16 = 1;
pub const BINARY_HEADER_SIZE: usize = 26;

pub mod opcode {
    pub const CREATE_NODE: u8 = 0x01;
    pub const SET_PROPERTY: u8 = 0x02;
    pub const SET_TEXT: u8 = 0x03;
    pub const INSERT_CHILD: u8 = 0x04;
    pub const REMOVE_CHILD: u8 = 0x05;
    pub const DROP_NODE: u8 = 0x06;
    pub const SET_STYLE: u8 = 0x07;
}

/// Strict decode failure. `field` names the offending part of the frame.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BinaryDecodeError {
    #[error("invalid ANB1 magic at offset {offset} (field={field})")]
    BadMagic { field: String, offset: usize },
    #[error("unsupported binary version {found}, expected {expected} (field={field})")]
    BadVersion {
        field: String,
        found: u16,
        expected: u16,
    },
    #[error("truncated {field} at offset {offset}")]
    Truncated { field: String, offset: usize },
    #[error("invalid UTF-8 in {field} at offset {offset}")]
    InvalidUtf8 { field: String, offset: usize },
    #[error("unknown opcode 0x{opcode:02x} in {field} at offset {offset}")]
    UnknownOpcode {
        field: String,
        opcode: u8,
        offset: usize,
    },
    #[error("invalid JSON value in {field} at offset {offset}: {detail}")]
    InvalidJsonValue {
        field: String,
        offset: usize,
        detail: String,
    },
    #[error("command count exceeds remaining bytes (field={field}, offset={offset})")]
    BadCommandCount { field: String, offset: usize },
    #[error("trailing bytes after batch at offset {offset} (field={field})")]
    TrailingBytes { field: String, offset: usize },
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn need(&self, size: usize, field: &str) -> Result<(), BinaryDecodeError> {
        if self.pos + size > self.bytes.len() {
            Err(BinaryDecodeError::Truncated {
                field: field.to_string(),
                offset: self.pos,
            })
        } else {
            Ok(())
        }
    }

    fn u8(&mut self, field: &str) -> Result<u8, BinaryDecodeError> {
        self.need(1, field)?;
        let value = self.bytes[self.pos];
        self.pos += 1;
        Ok(value)
    }

    fn u16(&mut self, field: &str) -> Result<u16, BinaryDecodeError> {
        self.need(2, field)?;
        let value = u16::from_le_bytes([self.bytes[self.pos], self.bytes[self.pos + 1]]);
        self.pos += 2;
        Ok(value)
    }

    fn u32(&mut self, field: &str) -> Result<u32, BinaryDecodeError> {
        self.need(4, field)?;
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&self.bytes[self.pos..self.pos + 4]);
        self.pos += 4;
        Ok(u32::from_le_bytes(buf))
    }

    fn u64(&mut self, field: &str) -> Result<u64, BinaryDecodeError> {
        self.need(8, field)?;
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&self.bytes[self.pos..self.pos + 8]);
        self.pos += 8;
        Ok(u64::from_le_bytes(buf))
    }

    fn string(&mut self, field: &str) -> Result<String, BinaryDecodeError> {
        let length = self.u32(&format!("{field}.length"))? as usize;
        self.need(length, field)?;
        let start = self.pos;
        self.pos += length;
        std::str::from_utf8(&self.bytes[start..start + length])
            .map(ToString::to_string)
            .map_err(|_| BinaryDecodeError::InvalidUtf8 {
                field: field.to_string(),
                offset: start,
            })
    }
}

/// Decode and strictly validate an ANB1 batch.
pub fn decode_binary_batch(bytes: &[u8]) -> Result<CommandBatch, BinaryDecodeError> {
    let mut reader = Reader::new(bytes);
    for (i, expected) in BINARY_MAGIC.iter().enumerate() {
        let actual = reader.u8("magic")?;
        if actual != *expected {
            return Err(BinaryDecodeError::BadMagic {
                field: "magic".to_string(),
                offset: i,
            });
        }
    }

    let version = reader.u16("version")?;
    if version != BINARY_VERSION {
        return Err(BinaryDecodeError::BadVersion {
            field: "version".to_string(),
            found: version,
            expected: BINARY_VERSION,
        });
    }

    let frame_id = reader.u64("frameId")?;
    let timestamp_ms = reader.u64("timestampMs")?;
    let count = reader.u32("commandCount")?;
    if count as usize > bytes.len().saturating_sub(reader.pos) {
        return Err(BinaryDecodeError::BadCommandCount {
            field: "commandCount".to_string(),
            offset: 22,
        });
    }

    let mut commands = Vec::with_capacity(count as usize);
    for index in 0..count {
        let where_ = format!("commands[{index}]");
        let opcode = reader.u8(&format!("{where_}.opcode"))?;
        let command = match opcode {
            opcode::CREATE_NODE => Command::CreateNode {
                id: reader.u64(&format!("{where_}.id"))?,
                node_type: reader.string(&format!("{where_}.nodeType"))?,
            },
            opcode::SET_PROPERTY => {
                let id = reader.u64(&format!("{where_}.id"))?;
                let key = reader.string(&format!("{where_}.key"))?;
                let offset = reader.pos;
                let value_json = reader.string(&format!("{where_}.value"))?;
                let value = serde_json::from_str(&value_json).map_err(|e| {
                    BinaryDecodeError::InvalidJsonValue {
                        field: format!("{where_}.value"),
                        offset,
                        detail: e.to_string(),
                    }
                })?;
                Command::SetProperty { id, key, value }
            }
            opcode::SET_TEXT => Command::SetTextContent {
                id: reader.u64(&format!("{where_}.id"))?,
                text: reader.string(&format!("{where_}.text"))?,
            },
            opcode::INSERT_CHILD => Command::InsertChild {
                parent_id: reader.u64(&format!("{where_}.parentId"))?,
                child_id: reader.u64(&format!("{where_}.childId"))?,
                index: reader.u32(&format!("{where_}.index"))? as usize,
            },
            opcode::REMOVE_CHILD => Command::RemoveChild {
                parent_id: reader.u64(&format!("{where_}.parentId"))?,
                child_id: reader.u64(&format!("{where_}.childId"))?,
            },
            opcode::DROP_NODE => Command::RemoveNode {
                id: reader.u64(&format!("{where_}.id"))?,
            },
            opcode::SET_STYLE => Command::SetStyle {
                id: reader.u64(&format!("{where_}.id"))?,
                key: reader.string(&format!("{where_}.key"))?,
                value: reader.string(&format!("{where_}.value"))?,
            },
            _ => {
                return Err(BinaryDecodeError::UnknownOpcode {
                    field: format!("{where_}.opcode"),
                    opcode,
                    offset: reader.pos - 1,
                });
            }
        };
        commands.push(command);
    }

    if reader.pos != bytes.len() {
        return Err(BinaryDecodeError::TrailingBytes {
            field: "trailing".to_string(),
            offset: reader.pos,
        });
    }

    Ok(CommandBatch {
        protocol_version: PROTOCOL_VERSION,
        frame_id,
        timestamp_ms,
        commands,
    })
}

/// Encode a batch into the versioned ANB1 wire format.
pub fn encode_binary_batch(batch: &CommandBatch) -> Result<Vec<u8>, String> {
    if batch.protocol_version != PROTOCOL_VERSION {
        return Err(format!(
            "Unsupported protocol version {}, expected {PROTOCOL_VERSION}",
            batch.protocol_version
        ));
    }
    let mut out = Vec::with_capacity(256);
    out.extend_from_slice(&BINARY_MAGIC);
    out.extend_from_slice(&BINARY_VERSION.to_le_bytes());
    out.extend_from_slice(&batch.frame_id.to_le_bytes());
    out.extend_from_slice(&batch.timestamp_ms.to_le_bytes());
    out.extend_from_slice(&(batch.commands.len() as u32).to_le_bytes());

    fn string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u32).to_le_bytes());
        out.extend_from_slice(value.as_bytes());
    }

    for command in &batch.commands {
        match command {
            Command::CreateNode { id, node_type } => {
                out.push(opcode::CREATE_NODE);
                out.extend_from_slice(&id.to_le_bytes());
                string(&mut out, node_type);
            }
            Command::RemoveNode { id } => {
                out.push(opcode::DROP_NODE);
                out.extend_from_slice(&id.to_le_bytes());
            }
            Command::InsertChild {
                parent_id,
                child_id,
                index,
            } => {
                out.push(opcode::INSERT_CHILD);
                out.extend_from_slice(&parent_id.to_le_bytes());
                out.extend_from_slice(&child_id.to_le_bytes());
                out.extend_from_slice(&(*index as u32).to_le_bytes());
            }
            Command::RemoveChild {
                parent_id,
                child_id,
            } => {
                out.push(opcode::REMOVE_CHILD);
                out.extend_from_slice(&parent_id.to_le_bytes());
                out.extend_from_slice(&child_id.to_le_bytes());
            }
            Command::SetProperty { id, key, value } => {
                out.push(opcode::SET_PROPERTY);
                out.extend_from_slice(&id.to_le_bytes());
                string(&mut out, key);
                let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
                string(&mut out, &json);
            }
            Command::SetStyle { id, key, value } => {
                out.push(opcode::SET_STYLE);
                out.extend_from_slice(&id.to_le_bytes());
                string(&mut out, key);
                string(&mut out, value);
            }
            Command::SetTextContent { id, text } => {
                out.push(opcode::SET_TEXT);
                out.extend_from_slice(&id.to_le_bytes());
                string(&mut out, text);
            }
        }
    }
    Ok(out)
}
