import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createBatch, type CommandBatch } from './protocol';
import {
  BINARY_VERSION,
  OPCODE,
  BinaryDecodeError,
  binaryRoundTrip,
  decodeBinaryBatch,
  encodeBinaryBatch,
} from './binary';

const GOLDEN_HEX = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'testing', 'fixtures', 'binary-batch.hex'),
  'utf-8',
).trim();

function goldenBytes(): Uint8Array {
  return new Uint8Array(Buffer.from(GOLDEN_HEX, 'hex'));
}

const goldenBatch = createBatch(1, 1000, [
  { kind: 'CreateNode', id: 1, nodeType: 'View' },
  { kind: 'CreateNode', id: 2, nodeType: 'Text' },
  { kind: 'InsertChild', parentId: 1, childId: 2, index: 0 },
  { kind: 'SetTextContent', id: 2, text: 'hello ☃' },
  { kind: 'SetStyle', id: 1, key: 'flexDirection', value: 'row' },
  { kind: 'SetProperty', id: 1, key: 'disabled', value: true },
  { kind: 'RemoveChild', parentId: 1, childId: 2 },
  { kind: 'RemoveNode', id: 2 },
]);

describe('binary mutation protocol (ANB1)', () => {
  it('round-trips every command kind without loss', () => {
    const decoded = binaryRoundTrip(goldenBatch);
    expect(decoded).toEqual(goldenBatch);
    expect(Array.from(decoded.commands.map((c) => c.kind))).toEqual([
      'CreateNode',
      'CreateNode',
      'InsertChild',
      'SetTextContent',
      'SetStyle',
      'SetProperty',
      'RemoveChild',
      'RemoveNode',
    ]);
  });

  it('decodes the shared golden vector byte-for-byte', () => {
    expect(decodeBinaryBatch(goldenBytes())).toEqual(goldenBatch);
    expect(Buffer.from(encodeBinaryBatch(goldenBatch)).toString('hex')).toBe(GOLDEN_HEX);
  });

  it('carries UTF-8 payloads that cannot cross a NUL-terminated C string', () => {
    const batch: CommandBatch = createBatch(2, 3, [
      { kind: 'SetTextContent', id: 7, text: 'a\u0000b ☃' },
    ]);
    const decoded = binaryRoundTrip(batch);
    expect(decoded.commands[0]).toEqual({ kind: 'SetTextContent', id: 7, text: 'a\u0000b ☃' });
  });

  it('rejects wrong magic and unsupported versions with field names', () => {
    const badMagic = goldenBytes();
    badMagic[0] = 0x58;
    expect(() => decodeBinaryBatch(badMagic)).toThrow(BinaryDecodeError);
    expect(() => decodeBinaryBatch(badMagic)).toThrow(/field=magic/);

    const badVersion = goldenBytes();
    new DataView(badVersion.buffer).setUint16(4, BINARY_VERSION + 1, true);
    expect(() => decodeBinaryBatch(badVersion)).toThrow(/Unsupported binary version/);
    expect(() => decodeBinaryBatch(badVersion)).toThrow(/field=version/);
  });

  it('rejects truncated frames and trailing bytes', () => {
    const full = goldenBytes();
    expect(() => decodeBinaryBatch(full.subarray(0, full.length - 1))).toThrow(/Truncated|field=/);
    const trailing = new Uint8Array(full.length + 1);
    trailing.set(full);
    trailing[full.length] = 0;
    expect(() => decodeBinaryBatch(trailing)).toThrow(/Trailing bytes/);
  });

  it('rejects unknown opcodes', () => {
    const bytes = goldenBytes();
    // First command begins after the 26-byte header.
    bytes[26] = 0xff;
    expect(() => decodeBinaryBatch(bytes)).toThrow(/Unknown opcode 0xff/);
  });

  it('rejects invalid UTF-8', () => {
    const batch = createBatch(1, 1, [{ kind: 'CreateNode', id: 1, nodeType: 'View' }]);
    const bytes = encodeBinaryBatch(batch);
    // nodeType length prefix starts at 26 + 1 + 8 = 35; payload at 39.
    bytes[39] = 0xff;
    expect(() => decodeBinaryBatch(bytes)).toThrow(/Invalid UTF-8/);
  });

  it('refuses node ids outside the safe JS integer range', () => {
    expect(() =>
      encodeBinaryBatch(
        createBatch(1, 1, [{ kind: 'CreateNode', id: Number.MAX_SAFE_INTEGER + 1, nodeType: 'View' }]),
      ),
    ).toThrow(/safe integer/);
  });

  it('keeps JSON fallback helpers independent from the binary codec', () => {
    expect(OPCODE.DropNode).toBe(0x06);
    expect(() => encodeBinaryBatch({ ...goldenBatch, protocolVersion: 99 })).toThrow(
      /Unsupported protocol version/,
    );
  });
});
