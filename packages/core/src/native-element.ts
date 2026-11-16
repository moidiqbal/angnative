/**
 * native-element.ts — Native element model (mirrors Rust `ViewNode` in
 * `crates/angnative-core/src/node.rs`).
 *
 * Every native node carries a unique 64-bit integer id (`node_id`), the
 * element tag, its Rust `NodeType`, parent/child links, properties, styles
 * and optional text content. This is the TS shadow tree that Angular's
 * custom renderer maintains before flushing mutations to the Rust host.
 */

import type { ElementContract } from '@angnative/contract/elements';
import type { StyleMap } from '@angnative/contract/style';

/** Unique 64-bit identifier for every native view node. */
export type NodeId = number;

/** Mirrors Rust `NodeType` (crates/angnative-core/src/node.rs). */
export type NativeNodeType =
  | 'View'
  | 'Text'
  | 'Button'
  | 'Image'
  | 'ScrollView'
  | 'TextInput'
  | 'Custom';

export interface NativeElementProps {
  readonly [key: string]: unknown;
}

/**
 * Represents an active native node in the shadow tree.
 */
export class NativeElement {
  readonly id: NodeId;
  readonly tag: string;
  readonly nodeType: NativeNodeType;
  parentId: NodeId | null;
  readonly children: NodeId[];
  properties: Map<string, unknown>;
  styles: Map<string, string>;
  textContent: string | null;

  constructor(id: NodeId, tag: string, nodeType: NativeNodeType) {
    this.id = id;
    this.tag = tag;
    this.nodeType = nodeType;
    this.parentId = null;
    this.children = [];
    this.properties = new Map();
    this.styles = new Map();
    this.textContent = null;
  }

  /** Apply a default style map (from the element contract). */
  applyDefaults(defaults: StyleMap): void {
    for (const [key, value] of Object.entries(defaults)) {
      if (!this.styles.has(key)) {
        this.styles.set(key, value);
      }
    }
  }

  setStyle(key: string, value: string): void {
    this.styles.set(key, value);
  }

  removeStyle(key: string): void {
    this.styles.delete(key);
  }

  setProperty(key: string, value: unknown): void {
    this.properties.set(key, value);
  }

  removeProperty(key: string): void {
    this.properties.delete(key);
  }

  addChild(childId: NodeId): void {
    if (!this.children.includes(childId)) {
      this.children.push(childId);
    }
  }

  removeChildId(childId: NodeId): void {
    const idx = this.children.indexOf(childId);
    if (idx >= 0) {
      this.children.splice(idx, 1);
    }
  }

  setTextContent(text: string): void {
    this.textContent = text;
  }
}

/**
 * Registry of all live native elements, keyed by node id. Provides the
 * element-factory SPI used by the renderer (create / lookup / remove).
 */
export class ElementRegistry {
  private readonly elements = new Map<NodeId, NativeElement>();
  private nextId = 1;

  /** Allocate the next unique node id. */
  allocateId(): NodeId {
    return this.nextId++;
  }

  /** Register a newly created element. */
  register(element: NativeElement): void {
    if (this.elements.has(element.id)) {
      throw new Error(`Element with id ${element.id} already registered`);
    }
    this.elements.set(element.id, element);
  }

  /** Look up an element by id. Throws if missing (loud failure). */
  get(id: NodeId): NativeElement {
    const element = this.elements.get(id);
    if (!element) {
      throw new Error(`No native element with id ${id}`);
    }
    return element;
  }

  has(id: NodeId): boolean {
    return this.elements.has(id);
  }

  /** Remove an element and detach it from its parent's child list. */
  remove(id: NodeId): void {
    const element = this.elements.get(id);
    if (!element) return;
    if (element.parentId !== null) {
      const parent = this.elements.get(element.parentId);
      if (parent) parent.removeChildId(id);
    }
    this.elements.delete(id);
  }

  /** All live elements. */
  get all(): IterableIterator<NativeElement> {
    return this.elements.values();
  }

  size(): number {
    return this.elements.size;
  }
}
