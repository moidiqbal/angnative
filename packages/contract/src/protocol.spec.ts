import { describe, it, expect } from 'vitest';
import {
  createBatch,
  encodeBatch,
  decodeBatch,
  roundTrip,
  PROTOCOL_VERSION,
} from './protocol';

describe('command protocol (TS↔Rust wire format)', () => {
  const batch = createBatch(1, 1000, [
    { kind: 'CreateNode', id: 1, nodeType: 'View' },
    { kind: 'CreateNode', id: 2, nodeType: 'Text' },
    {
      kind: 'InsertChild',
      parentId: 1,
      childId: 2,
      index: 0,
    },
    { kind: 'SetTextContent', id: 2, text: 'hello' },
    { kind: 'SetStyle', id: 1, key: 'flexDirection', value: 'row' },
    { kind: 'SetProperty', id: 1, key: 'disabled', value: true },
    { kind: 'RemoveChild', parentId: 1, childId: 2 },
    { kind: 'RemoveNode', id: 2 },
  ]);

  it('round-trips through JSON without loss', () => {
    const json = encodeBatch(batch);
    const decoded = decodeBatch(json);
    expect(decoded.protocolVersion).toBe(PROTOCOL_VERSION);
    expect(decoded.frameId).toBe(1);
    expect(decoded.timestampMs).toBe(1000);
    expect(decoded.commands).toHaveLength(8);
    expect(decoded.commands[0]).toEqual({
      kind: 'CreateNode',
      id: 1,
      nodeType: 'View',
    });
    expect(decoded.commands[3]).toEqual({
      kind: 'SetTextContent',
      id: 2,
      text: 'hello',
    });
    expect(decoded.commands[5]).toEqual({
      kind: 'SetProperty',
      id: 1,
      key: 'disabled',
      value: true,
    });
  });

  it('roundTrip helper is identity', () => {
    expect(roundTrip(batch)).toEqual(batch);
  });

  it('rejects unsupported protocol versions loudly', () => {
    const bad = createBatch(1, 1, []);
    const json = JSON.stringify({ ...bad, protocolVersion: 99 });
    expect(() => decodeBatch(json)).toThrow(/Unsupported protocol version/);
  });

  it('rejects unknown command kinds', () => {
    const json = JSON.stringify({
      protocolVersion: PROTOCOL_VERSION,
      frameId: 1,
      timestampMs: 1,
      commands: [{ kind: 'BogusCommand', id: 1 }],
    });
    expect(() => decodeBatch(json)).toThrow(/Unknown command kind/);
  });

  it('rejects missing required fields', () => {
    const json = JSON.stringify({
      protocolVersion: PROTOCOL_VERSION,
      frameId: 1,
      timestampMs: 1,
      commands: [{ kind: 'CreateNode', nodeType: 'View' }],
    });
    expect(() => decodeBatch(json)).toThrow(/missing field/);
  });

  it('rejects non-finite frameId / timestampMs', () => {
    const json = JSON.stringify({
      protocolVersion: PROTOCOL_VERSION,
      frameId: NaN,
      timestampMs: 1,
      commands: [],
    });
    expect(() => decodeBatch(json)).toThrow(/frameId/);
  });
});
