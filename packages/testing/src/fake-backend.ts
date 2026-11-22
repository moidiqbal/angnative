/**
 * fake-backend.ts — TS fake backend runner for the shared conformance suite.
 *
 * Applies command-protocol commands directly to an ElementRegistry (pure
 * TypeScript shadow tree), mirroring what the Rust host does when it decodes
 * a command batch. Results are compared against fixture `expect` blocks.
 *
 * This is the TS implementation of the TS↔Rust boundary; the Rust host
 * (angnative-host-core) must produce identical tree state from the same
 * fixtures (Task R4 — the keystone).
 */

import { ElementRegistry, NativeElement, type NodeId } from '@angnative/core/native-element';
import { parseStyleValue } from '@angnative/contract/style';
import type { Command } from '@angnative/contract/protocol';

/** Snapshot of a node's observable state. */
export interface NodeSnapshot {
  readonly id: NodeId;
  readonly tag: string;
  readonly nodeType: string;
  readonly parentId: NodeId | null;
  readonly children: readonly NodeId[];
  readonly properties: ReadonlyMap<string, unknown>;
  readonly styles: ReadonlyMap<string, string>;
  readonly textContent: string | null;
}

/** Result of running commands through the fake backend. */
export interface FakeBackendResult {
  readonly batches: readonly string[][]; // mutation kinds per batch, in order
  readonly nodes: ReadonlyMap<NodeId, NodeSnapshot>;
  /**
   * How many times each node id was created. A node whose native view slot
   * survives a detach/reattach cycle is created exactly once; a host that
   * destroys and recreates the slot will report a count greater than one.
   */
  readonly createCounts: ReadonlyMap<NodeId, number>;
}
function tagToNodeType(tag: string): string {
  switch (tag.toLowerCase()) {
    case 'na-view':
    case 'native-view':
    case 'view':
    case 'div':
      return 'View';
    case 'na-text':
    case 'native-text':
    case 'text':
    case 'span':
    case 'p':
      return 'Text';
    case 'na-button':
    case 'native-button':
    case 'button':
      return 'Button';
    case 'na-image':
    case 'native-image':
    case 'image':
    case 'img':
      return 'Image';
    case 'na-scroll-view':
    case 'native-scroll':
    case 'scroll':
    case 'scrollview':
      return 'ScrollView';
    case 'na-text-input':
    case 'native-input':
    case 'input':
      return 'TextInput';
    default:
      return 'Custom';
  }
}

/** Node types recognized by both the TS layer and the Rust host. */
const KNOWN_NODE_TYPES = new Set([
  'View',
  'Text',
  'Button',
  'Image',
  'ScrollView',
  'TextInput',
  'Custom',
]);

/**
 * The fake backend. Directly manages a shadow tree; each command mutates the
 * tree and is recorded as a mutation kind. No renderer auto-flush, no Rust.
 */
export class FakeBackend {
  protected readonly registry = new ElementRegistry();
  private readonly batches: string[][] = [];
  private currentBatch: string[] = [];
  private readonly createCounts = new Map<NodeId, number>();

  /** Execute a single command. Returns the mutation kind recorded. */
  runCommand(cmd: Command): string {
    const kind = this.apply(cmd);
    this.currentBatch.push(kind);
    return kind;
  }

  /** Execute a command batch (flushes the current accumulated batch). */
  runBatch(commands: readonly Command[]): void {
    for (const cmd of commands) {
      this.runCommand(cmd);
    }
    this.flushBatch();
  }

  /** Flush the accumulated mutation-kind batch. */
  flushBatch(): void {
    if (this.currentBatch.length > 0) {
      this.batches.push([...this.currentBatch]);
      this.currentBatch = [];
    }
  }

  /** Snapshot the full shadow tree. */
  snapshot(): FakeBackendResult {
    this.flushBatch();
    const nodes = new Map<NodeId, NodeSnapshot>();
    for (const el of this.registry.all) {
      nodes.set(el.id, this.snapshotNode(el));
    }
    return { batches: [...this.batches], nodes, createCounts: new Map(this.createCounts) };
  }

  /** Look up a single node snapshot. Returns undefined if the node was removed. */
  getNode(id: NodeId): NodeSnapshot | undefined {
    if (!this.registry.has(id)) return undefined;
    return this.snapshotNode(this.registry.get(id));
  }

  /** All mutation kinds emitted so far, in order (flattened across batches). */
  mutationKinds(): readonly string[] {
    this.flushBatch();
    return this.batches.flatMap((b) => b);
  }

  /** Number of live nodes. */
  size(): number {
    return this.registry.size();
  }

  protected apply(cmd: Command): string {
    const r = this.registry;

    switch (cmd.kind) {
      case 'CreateNode': {
        if (cmd.id === undefined || cmd.id === null) {
          throw new Error('Invalid CreateNode: missing field "id"');
        }
        if (!KNOWN_NODE_TYPES.has(cmd.nodeType)) {
          throw new Error(`Unknown node type: ${cmd.nodeType}`);
        }
        const el = new NativeElement(cmd.id, cmd.nodeType.toLowerCase(), cmd.nodeType as any);
        r.registerWithId(el);
        this.createCounts.set(cmd.id, (this.createCounts.get(cmd.id) ?? 0) + 1);
        return 'CreateNode';
      }
      case 'RemoveNode': {
        r.remove(cmd.id);
        return 'RemoveNode';
      }
      case 'InsertChild': {
        const parent = r.get(cmd.parentId);
        const child = r.get(cmd.childId);

        // Detach from previous parent if re-parenting.
        if (child.parentId !== null && child.parentId !== parent.id) {
          const oldParent = r.get(child.parentId);
          oldParent.removeChildId(child.id);
        }

        child.parentId = parent.id;

        // Insert at index, or append if index is at/beyond the end.
        const idx = Math.min(cmd.index, parent.children.length);
        parent.children.splice(idx, 0, child.id);

        return 'InsertChild';
      }
      case 'RemoveChild': {
        const parent = r.get(cmd.parentId);
        const child = r.get(cmd.childId);
        // M0 fix: non-member removeChild must throw — child must belong to parent.
        if (child.parentId !== parent.id) {
          throw new Error(
            `Element ${cmd.childId} is not a child of element ${cmd.parentId}`,
          );
        }
        child.parentId = null;
        parent.removeChildId(cmd.childId);
        return 'RemoveChild';
      }
      case 'SetProperty': {
        const el = r.get(cmd.id);
        // M0 fix: null value removes the property.
        if (cmd.value === null) {
          el.removeProperty(cmd.key);
        } else {
          el.setProperty(cmd.key, cmd.value);
        }
        return 'SetProperty';
      }
      case 'SetStyle': {
        const el = r.get(cmd.id);
        // Validate + canonicalize against the style table (§5.3: unknown → throw).
        const canonical = parseStyleValue(cmd.key, cmd.value);
        el.setStyle(cmd.key, canonical);
        return 'SetStyle';
      }
      case 'SetTextContent': {
        const el = r.get(cmd.id);
        el.setTextContent(cmd.text);
        return 'SetTextContent';
      }
    }
  }

  private snapshotNode(el: NativeElement): NodeSnapshot {
    return {
      id: el.id,
      tag: el.tag,
      nodeType: el.nativeNodeType,
      parentId: el.parentId,
      children: [...el.children],
      properties: new Map(el.properties),
      styles: new Map(el.styles),
      textContent: el.textContent,
    };
  }
}
