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
  /**
   * DOM-compatible numeric node kind used by Angular internals that test
   * `node.nodeType === 1` (notably the animation engine). Native view types
   * stay available as `nativeNodeType` for the command protocol and backends.
   */
  readonly nodeType: 1 | 3 | 8;
  readonly nativeNodeType: NativeNodeType;
  parentId: NodeId | null;
  readonly children: NodeId[];
  properties: Map<string, unknown>;
  styles: Map<string, string>;
  /**
   * Flat inline-style mirror for Angular internals that touch `element.style`
   * directly (for example animation cleanup). The renderer's `styles` map is
   * authoritative for the native tree; this object has no closures or cycles.
   */
  readonly style: Record<string, string>;
  textContent: string | null;
  /** Numeric animation host id set by the renderer that created this node. */
  animationHostId: number;

  constructor(id: NodeId, tag: string, nodeType: NativeNodeType) {
    this.id = id;
    this.tag = tag;
    this.nativeNodeType = nodeType;
    this.nodeType = tag === '#text' ? 3 : tag === '#comment' ? 8 : 1;
    this.animationHostId = 0;
    this.parentId = null;
    this.children = [];
    this.properties = new Map();
    this.styles = new Map();
    this.style = {};
    this.textContent = null;
  }

  /** Apply a default style map (from the element contract). */
  applyDefaults(defaults: StyleMap): void {
    for (const [key, value] of Object.entries(defaults)) {
      if (!this.styles.has(key)) {
        this.styles.set(key, value);
        this.style[key] = value;
      }
    }
  }

  setStyle(key: string, value: string): void {
    this.styles.set(key, value);
    this.style[key] = value;
  }

  /**
   * Web Animations API entry point used by Angular's `WebAnimationsDriver`.
   * The implementation is a flat global hook keyed by numeric ids; storing a
   * renderer closure here would create a QuickJS object cycle.
   */
  animate(keyframes: unknown, options?: unknown): unknown {
    const globals = globalThis as unknown as Record<string, unknown>;
    const hook = globals['__angnativeAnimateElement'] as
      | ((hostId: number, nodeId: number, keyframes: unknown, options?: unknown) => unknown)
      | undefined;
    if (typeof hook !== 'function') return null;
    return hook(this.animationHostId, this.id, keyframes, options);
  }

  removeStyle(key: string): void {
    this.styles.delete(key);
    delete this.style[key];
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

  /** Register an element with an explicit id (used by the fake backend / Rust host mirror). */
  registerWithId(element: NativeElement): void {
    if (this.elements.has(element.id)) {
      throw new Error(`Element with id ${element.id} already registered`);
    }
    this.elements.set(element.id, element);
    if (element.id >= this.nextId) {
      this.nextId = element.id + 1;
    }
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
