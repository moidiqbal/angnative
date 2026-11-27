/**
 * binary.ts — versioned ANB1 binary mutation protocol.
 *
 * This is an opt-in companion to the JSON wire format in protocol.ts. Both
 * formats decode to the same CommandBatch model; JSON remains the fallback.
 *
 * Framing (all integers little-endian):
 *   magic "ANB1" | u16 version | u64 frameId | u64 timestampMs
 *   | u32 commandCount | commands...
 *
 * Commands:
 *   0x01 CreateNode   { id:u64, nodeType:string }
 *   0x02 SetProperty  { id:u64, key:string, valueJson:string }
 *   0x03 SetText      { id:u64, text:string }
 *   0x04 InsertChild  { parentId:u64, childId:u64, index:u32 }
 *   0x05 RemoveChild  { parentId:u64, childId:u64 }
 *   0x06 DropNode     { id:u64 }
 *   0x07 SetStyle     { id:u64, key:string, value:string }
 *
 * Strings are u32 byte length followed by UTF-8 bytes. Node ids, frame ids
 * and timestamps are u64 but must be safe JS integers (< 2^53).
 */

import {
  PROTOCOL_VERSION,
  type Command,
  type CommandBatch,
  type JsonValue,
} from './protocol';

export const BINARY_MAGIC = 'ANB1';
export const BINARY_VERSION = 1;
export const BINARY_HEADER_SIZE = 26;

/** Canonical opcode numbers shared with the Rust decoder. */
export const OPCODE = {
  CreateNode: 0x01,
  SetProperty: 0x02,
  SetText: 0x03,
  InsertChild: 0x04,
  RemoveChild: 0x05,
  DropNode: 0x06,
  SetStyle: 0x07,
} as const;

/** A decode failure names the field and byte offset that caused it. */
export class BinaryDecodeError extends Error {
  readonly field: string;
  readonly offset: number;

  constructor(message: string, field: string, offset: number) {
    super(`${message} (field=${field}, offset=${offset})`);
    this.name = 'BinaryDecodeError';
    this.field = field;
    this.offset = offset;
  }
}

/* ---------- UTF-8 (no TextEncoder/Decoder dependency in QuickJS) ---------- */

function utf8Encode(value: string): Uint8Array {
  const out: number[] = [];
  for (let i = 0; i < value.length; i++) {
    const unit = value.charCodeAt(i);
    if (unit < 0x80) {
      out.push(unit);
    } else if (unit < 0x800) {
      out.push(0xc0 | (unit >> 6), 0x80 | (unit & 0x3f));
    } else if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(i + 1);
      if (next < 0xdc00 || next > 0xdfff) {
        throw new Error('Cannot encode lone UTF-16 surrogate');
      }
      i++;
      const cp = 0x10000 + ((unit - 0xd800) << 10) + (next - 0xdc00);
      out.push(
        0xf0 | (cp >> 18),
        0x80 | ((cp >> 12) & 0x3f),
        0x80 | ((cp >> 6) & 0x3f),
        0x80 | (cp & 0x3f),
      );
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      throw new Error('Cannot encode lone UTF-16 surrogate');
    } else {
      out.push(0xe0 | (unit >> 12), 0x80 | ((unit >> 6) & 0x3f), 0x80 | (unit & 0x3f));
    }
  }
  return new Uint8Array(out);
}

function utf8Decode(bytes: Uint8Array, field: string, offset: number): string {
  let out = '';
  let i = 0;
  while (i < bytes.length) {
    const b0 = bytes[i]!;
    if (b0 < 0x80) {
      out += String.fromCharCode(b0);
      i++;
      continue;
    }
    let cp: number;
    let width: number;
    if (b0 >= 0xc2 && b0 <= 0xdf) {
      cp = b0 & 0x1f;
      width = 2;
    } else if (b0 >= 0xe0 && b0 <= 0xef) {
      cp = b0 & 0x0f;
      width = 3;
    } else if (b0 >= 0xf0 && b0 <= 0xf4) {
      cp = b0 & 0x07;
      width = 4;
    } else {
      throw new BinaryDecodeError('Invalid UTF-8 lead byte', field, offset + i);
    }
    if (i + width > bytes.length) {
      throw new BinaryDecodeError('Truncated UTF-8 sequence', field, offset + i);
    }
    for (let j = 1; j < width; j++) {
      const bj = bytes[i + j]!;
      if ((bj & 0xc0) !== 0x80) {
        throw new BinaryDecodeError('Invalid UTF-8 continuation byte', field, offset + i + j);
      }
      cp = (cp << 6) | (bj & 0x3f);
    }
    const minimal =
      (width === 2 && cp >= 0x80) ||
      (width === 3 && cp >= 0x800) ||
      (width === 4 && cp >= 0x10000);
    if (!minimal || cp > 0x10ffff || (cp >= 0xd800 && cp <= 0xdfff)) {
      throw new BinaryDecodeError('Invalid UTF-8 code point', field, offset + i);
    }
    if (cp > 0xffff) {
      cp -= 0x10000;
      out += String.fromCharCode(0xd800 | (cp >> 10), 0xdc00 | (cp & 0x3ff));
    } else {
      out += String.fromCharCode(cp);
    }
    i += width;
  }
  return out;
}

/* ---------- writer ---------- */

class Writer {
  private bytes: Uint8Array;
  private view: DataView;
  pos = 0;

  constructor(initial: number) {
    this.bytes = new Uint8Array(initial);
    this.view = new DataView(this.bytes.buffer);
  }

  private ensure(extra: number): void {
    if (this.pos + extra <= this.bytes.length) return;
    const grown = new Uint8Array(Math.max(this.bytes.length * 2, this.pos + extra));
    grown.set(this.bytes);
    this.bytes = grown;
    this.view = new DataView(grown.buffer);
  }

  u8(value: number): void {
    this.ensure(1);
    this.view.setUint8(this.pos++, value);
  }

  u16(value: number): void {
    this.ensure(2);
    this.view.setUint16(this.pos, value, true);
    this.pos += 2;
  }

  u32(value: number): void {
    this.ensure(4);
    this.view.setUint32(this.pos, value, true);
    this.pos += 4;
  }

  u64(value: number, field: string): void {
    if (!Number.isSafeInteger(value) || value < 0) {
      throw new Error(`${field} must be a non-negative safe integer, got ${value}`);
    }
    this.ensure(8);
    this.view.setBigUint64(this.pos, BigInt(value), true);
    this.pos += 8;
  }

  string(value: string, field: string): void {
    const encoded = utf8Encode(value);
    if (encoded.length > 0xffffffff) {
      throw new Error(`${field} is too long for u32 length prefix`);
    }
    this.u32(encoded.length);
    this.ensure(encoded.length);
    this.bytes.set(encoded, this.pos);
    this.pos += encoded.length;
  }

  finish(): Uint8Array {
    return this.bytes.slice(0, this.pos);
  }
}

function requireSafeU64(value: number, field: string): number {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new Error(`${field} must be a non-negative safe integer, got ${value}`);
  }
  return value;
}

/** Encode a batch into the versioned ANB1 wire format. */
export function encodeBinaryBatch(batch: CommandBatch): Uint8Array {
  if (batch.protocolVersion !== PROTOCOL_VERSION) {
    throw new Error(
      `Unsupported protocol version ${batch.protocolVersion}, expected ${PROTOCOL_VERSION}`,
    );
  }
  const writer = new Writer(256);
  for (const char of BINARY_MAGIC) writer.u8(char.charCodeAt(0));
  writer.u16(BINARY_VERSION);
  writer.u64(requireSafeU64(batch.frameId, 'frameId'), 'frameId');
  writer.u64(requireSafeU64(batch.timestampMs, 'timestampMs'), 'timestampMs');
  writer.u32(batch.commands.length);

  for (const command of batch.commands) {
    switch (command.kind) {
      case 'CreateNode':
        writer.u8(OPCODE.CreateNode);
        writer.u64(command.id, 'CreateNode.id');
        writer.string(command.nodeType, 'CreateNode.nodeType');
        break;
      case 'SetProperty': {
        writer.u8(OPCODE.SetProperty);
        writer.u64(command.id, 'SetProperty.id');
        writer.string(command.key, 'SetProperty.key');
        const json = JSON.stringify(command.value);
        if (json === undefined) {
          throw new Error('SetProperty.value must be JSON-serializable');
        }
        writer.string(json, 'SetProperty.value');
        break;
      }
      case 'SetTextContent':
        writer.u8(OPCODE.SetText);
        writer.u64(command.id, 'SetTextContent.id');
        writer.string(command.text, 'SetTextContent.text');
        break;
      case 'InsertChild':
        writer.u8(OPCODE.InsertChild);
        writer.u64(command.parentId, 'InsertChild.parentId');
        writer.u64(command.childId, 'InsertChild.childId');
        writer.u32(requireSafeU32(command.index, 'InsertChild.index'));
        break;
      case 'RemoveChild':
        writer.u8(OPCODE.RemoveChild);
        writer.u64(command.parentId, 'RemoveChild.parentId');
        writer.u64(command.childId, 'RemoveChild.childId');
        break;
      case 'RemoveNode':
        writer.u8(OPCODE.DropNode);
        writer.u64(command.id, 'RemoveNode.id');
        break;
      case 'SetStyle':
        writer.u8(OPCODE.SetStyle);
        writer.u64(command.id, 'SetStyle.id');
        writer.string(command.key, 'SetStyle.key');
        writer.string(command.value, 'SetStyle.value');
        break;
      default:
        throw new Error(`Unknown command kind: ${JSON.stringify(command)}`);
    }
  }
  return writer.finish();
}

function requireSafeU32(value: number, field: string): number {
  if (!Number.isSafeInteger(value) || value < 0 || value > 0xffffffff) {
    throw new Error(`${field} must be a u32, got ${value}`);
  }
  return value;
}

/* ---------- reader ---------- */

class Reader {
  readonly view: DataView;
  pos = 0;

  constructor(bytes: Uint8Array) {
    this.view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  }

  get length(): number {
    return this.view.byteLength;
  }

  need(size: number, field: string): void {
    if (this.pos + size > this.length) {
      throw new BinaryDecodeError(`Truncated ${field}`, field, this.pos);
    }
  }

  u8(field: string): number {
    this.need(1, field);
    return this.view.getUint8(this.pos++);
  }

  u16(field: string): number {
    this.need(2, field);
    const value = this.view.getUint16(this.pos, true);
    this.pos += 2;
    return value;
  }

  u32(field: string): number {
    this.need(4, field);
    const value = this.view.getUint32(this.pos, true);
    this.pos += 4;
    return value;
  }

  u64(field: string): number {
    this.need(8, field);
    const value = this.view.getBigUint64(this.pos, true);
    this.pos += 8;
    if (value > BigInt(Number.MAX_SAFE_INTEGER)) {
      throw new BinaryDecodeError('u64 exceeds safe JS integer range', field, this.pos - 8);
    }
    return Number(value);
  }

  string(field: string): string {
    const length = this.u32(`${field}.length`);
    this.need(length, field);
    const start = this.pos;
    const bytes = new Uint8Array(this.view.buffer, this.view.byteOffset + start, length);
    this.pos += length;
    return utf8Decode(bytes, field, start);
  }
}

/** Decode and strictly validate an ANB1 batch. */
export function decodeBinaryBatch(input: Uint8Array | ArrayBufferLike): CommandBatch {
  const bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
  const reader = new Reader(bytes);

  for (let i = 0; i < BINARY_MAGIC.length; i++) {
    if (reader.u8('magic') !== BINARY_MAGIC.charCodeAt(i)) {
      throw new BinaryDecodeError('Invalid ANB1 magic', 'magic', i);
    }
  }
  const version = reader.u16('version');
  if (version !== BINARY_VERSION) {
    throw new BinaryDecodeError(
      `Unsupported binary version ${version}, expected ${BINARY_VERSION}`,
      'version',
      4,
    );
  }
  const frameId = reader.u64('frameId');
  const timestampMs = reader.u64('timestampMs');
  const count = reader.u32('commandCount');
  if (count > Math.floor((reader.length - reader.pos) / 1)) {
    throw new BinaryDecodeError('Command count exceeds remaining bytes', 'commandCount', 22);
  }

  const commands: Command[] = [];
  for (let i = 0; i < count; i++) {
    const where = `commands[${i}]`;
    const opcode = reader.u8(`${where}.opcode`);
    switch (opcode) {
      case OPCODE.CreateNode:
        commands.push({
          kind: 'CreateNode',
          id: reader.u64(`${where}.id`),
          nodeType: reader.string(`${where}.nodeType`),
        });
        break;
      case OPCODE.SetProperty: {
        const id = reader.u64(`${where}.id`);
        const key = reader.string(`${where}.key`);
        const valueJson = reader.string(`${where}.value`);
        let value: JsonValue;
        try {
          value = JSON.parse(valueJson) as JsonValue;
        } catch {
          throw new BinaryDecodeError('Invalid JSON value', `${where}.value`, reader.pos);
        }
        commands.push({ kind: 'SetProperty', id, key, value });
        break;
      }
      case OPCODE.SetText:
        commands.push({
          kind: 'SetTextContent',
          id: reader.u64(`${where}.id`),
          text: reader.string(`${where}.text`),
        });
        break;
      case OPCODE.InsertChild:
        commands.push({
          kind: 'InsertChild',
          parentId: reader.u64(`${where}.parentId`),
          childId: reader.u64(`${where}.childId`),
          index: reader.u32(`${where}.index`),
        });
        break;
      case OPCODE.RemoveChild:
        commands.push({
          kind: 'RemoveChild',
          parentId: reader.u64(`${where}.parentId`),
          childId: reader.u64(`${where}.childId`),
        });
        break;
      case OPCODE.DropNode:
        commands.push({ kind: 'RemoveNode', id: reader.u64(`${where}.id`) });
        break;
      case OPCODE.SetStyle:
        commands.push({
          kind: 'SetStyle',
          id: reader.u64(`${where}.id`),
          key: reader.string(`${where}.key`),
          value: reader.string(`${where}.value`),
        });
        break;
      default:
        throw new BinaryDecodeError(`Unknown opcode 0x${opcode.toString(16)}`, `${where}.opcode`, reader.pos - 1);
    }
  }
  if (reader.pos !== reader.length) {
    throw new BinaryDecodeError('Trailing bytes after batch', 'trailing', reader.pos);
  }
  return {
    protocolVersion: PROTOCOL_VERSION,
    frameId,
    timestampMs,
    commands,
  };
}

/** Round-trip a batch through the binary wire format (useful in tests). */
export function binaryRoundTrip(batch: CommandBatch): CommandBatch {
  return decodeBinaryBatch(encodeBinaryBatch(batch));
}
