/**
 * protocol.ts — the versioned TS↔Rust command protocol.
 *
 * This is the wire format between Angular's TypeScript layer
 * (@angnative/core renderer) and the Rust host (angnative-host-core).
 * Every command maps 1:1 to a `ViewMutation` in
 * `crates/angnative-core/src/mutation.rs` — same names, same fields,
 * same JSON shape — so a command batch round-trips through Rust
 * serde without an intermediate schema.
 *
 * Commands are versioned. A batch with an unsupported
 * `protocolVersion` is rejected loudly rather than silently misread.
 */

export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { readonly [key: string]: JsonValue };

export const PROTOCOL_VERSION = 1;

/* ---------- commands ---------- */

export interface CommandCreateNode {
  readonly kind: 'CreateNode';
  /** 64-bit node id, unique across the view tree. */
  readonly id: number;
  readonly nodeType: string;
}

export interface CommandRemoveNode {
  readonly kind: 'RemoveNode';
  readonly id: number;
}

export interface CommandInsertChild {
  readonly kind: 'InsertChild';
  readonly parentId: number;
  readonly childId: number;
  readonly index: number;
}

export interface CommandRemoveChild {
  readonly kind: 'RemoveChild';
  readonly parentId: number;
  readonly childId: number;
}

export interface CommandSetProperty {
  readonly kind: 'SetProperty';
  readonly id: number;
  readonly key: string;
  readonly value: JsonValue;
}

export interface CommandSetStyle {
  readonly kind: 'SetStyle';
  readonly id: number;
  readonly key: string;
  readonly value: string;
}

export interface CommandSetTextContent {
  readonly kind: 'SetTextContent';
  readonly id: number;
  readonly text: string;
}

export type Command =
  | CommandCreateNode
  | CommandRemoveNode
  | CommandInsertChild
  | CommandRemoveChild
  | CommandSetProperty
  | CommandSetStyle
  | CommandSetTextContent;

export const COMMAND_KINDS = new Set([
  'CreateNode',
  'RemoveNode',
  'InsertChild',
  'RemoveChild',
  'SetProperty',
  'SetStyle',
  'SetTextContent',
]);

/* ---------- batch ---------- */

export interface CommandBatch {
  readonly protocolVersion: number;
  readonly frameId: number;
  readonly timestampMs: number;
  readonly commands: readonly Command[];
}

export function createBatch(
  frameId: number,
  timestampMs: number,
  commands: readonly Command[] = [],
): CommandBatch {
  return {
    protocolVersion: PROTOCOL_VERSION,
    frameId,
    timestampMs,
    commands: [...commands],
  };
}

/* ---------- encode / decode ---------- */

/** Serialize a batch to the JSON wire format sent over FFI to Rust. */
export function encodeBatch(batch: CommandBatch): string {
  return JSON.stringify(batch);
}

/**
 * Parse and validate a batch from the JSON wire format.
 * Throws on unknown versions, unknown command kinds, or malformed commands.
 */
export function decodeBatch(json: string | unknown): CommandBatch {
  let raw: unknown;
  if (typeof json === 'string') {
    raw = JSON.parse(json);
  } else {
    raw = json;
  }

  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) {
    throw new Error('Invalid command batch: expected an object');
  }
  const batch = raw as Record<string, unknown>;

  if (batch.protocolVersion !== PROTOCOL_VERSION) {
    throw new Error(
      `Unsupported protocol version ${JSON.stringify(
        batch.protocolVersion,
      )}, expected ${PROTOCOL_VERSION}`,
    );
  }

  if (typeof batch.frameId !== 'number' || !Number.isFinite(batch.frameId)) {
    throw new Error('Invalid command batch: frameId must be a finite number');
  }
  if (typeof batch.timestampMs !== 'number' || !Number.isFinite(batch.timestampMs)) {
    throw new Error('Invalid command batch: timestampMs must be a finite number');
  }

  const commandsRaw = batch.commands;
  if (!Array.isArray(commandsRaw)) {
    throw new Error('Invalid command batch: commands must be an array');
  }

  const commands: Command[] = [];
  for (const entry of commandsRaw) {
    commands.push(decodeCommand(entry));
  }

  return {
    protocolVersion: PROTOCOL_VERSION,
    frameId: batch.frameId,
    timestampMs: batch.timestampMs,
    commands,
  };
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function requireField<T>(
  obj: Record<string, unknown>,
  key: string,
  where: string,
): T {
  const value = obj[key];
  if (value === undefined) {
    throw new Error(`Invalid ${where}: missing field "${key}"`);
  }
  return value as T;
}

function requireString(obj: Record<string, unknown>, key: string, where: string): string {
  const value = requireField(obj, key, where);
  if (typeof value !== 'string') {
    throw new Error(`Invalid ${where}: field "${key}" must be a string`);
  }
  return value;
}

function requireFinite(obj: Record<string, unknown>, key: string, where: string): number {
  const value = requireField(obj, key, where);
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    throw new Error(`Invalid ${where}: field "${key}" must be a finite number`);
  }
  return value;
}

function requireNonNegativeInteger(
  obj: Record<string, unknown>,
  key: string,
  where: string,
): number {
  const value = requireFinite(obj, key, where);
  if (!Number.isInteger(value) || value < 0) {
    throw new Error(`Invalid ${where}: field "${key}" must be a non-negative integer`);
  }
  return value;
}

function decodeCommand(raw: unknown): Command {
  if (!isPlainObject(raw)) {
    throw new Error('Invalid command: expected an object');
  }
  const cmd = raw as Record<string, unknown>;
  const kind = requireString(cmd, 'kind', 'command');
  if (!COMMAND_KINDS.has(kind)) {
    throw new Error(`Unknown command kind: ${JSON.stringify(kind)}`);
  }

  switch (kind) {
    case 'CreateNode':
      return {
        kind,
        id: requireNonNegativeInteger(cmd, 'id', 'CreateNode'),
        nodeType: requireString(cmd, 'nodeType', 'CreateNode'),
      };
    case 'RemoveNode':
      return { kind, id: requireNonNegativeInteger(cmd, 'id', 'RemoveNode') };
    case 'InsertChild':
      return {
        kind,
        parentId: requireNonNegativeInteger(cmd, 'parentId', 'InsertChild'),
        childId: requireNonNegativeInteger(cmd, 'childId', 'InsertChild'),
        index: requireNonNegativeInteger(cmd, 'index', 'InsertChild'),
      };
    case 'RemoveChild':
      return {
        kind,
        parentId: requireNonNegativeInteger(cmd, 'parentId', 'RemoveChild'),
        childId: requireNonNegativeInteger(cmd, 'childId', 'RemoveChild'),
      };
    case 'SetProperty': {
      const value = requireField<JsonValue>(cmd, 'value', 'SetProperty');
      return {
        kind,
        id: requireNonNegativeInteger(cmd, 'id', 'SetProperty'),
        key: requireString(cmd, 'key', 'SetProperty'),
        value,
      };
    }
    case 'SetStyle':
      return {
        kind,
        id: requireNonNegativeInteger(cmd, 'id', 'SetStyle'),
        key: requireString(cmd, 'key', 'SetStyle'),
        value: requireString(cmd, 'value', 'SetStyle'),
      };
    case 'SetTextContent':
      return {
        kind,
        id: requireNonNegativeInteger(cmd, 'id', 'SetTextContent'),
        text: requireString(cmd, 'text', 'SetTextContent'),
      };
    default:
      // unreachable — COMMAND_KINDS is the source of truth
      throw new Error(`Unknown command kind: ${JSON.stringify(kind)}`);
  }
}

/** Round-trip a batch through encode/decode (useful in tests). */
export function roundTrip(batch: CommandBatch): CommandBatch {
  return decodeBatch(encodeBatch(batch));
}
