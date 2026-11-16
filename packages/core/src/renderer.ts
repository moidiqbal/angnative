/**
 * renderer.ts — AngNative custom Angular `Renderer2` (Architecture §3.1).
 *
 * One renderer, written once, above the element-factory SPI
 * (`ElementRegistry`). No per-platform forks — the same renderer is used by
 * the TypeScript fake backend and by the Rust host (Task T2).
 *
 * Every DOM-ish operation Angular performs is translated into a
 * `ViewMutation` (see `crates/angnative-core/src/mutation.rs` / the command
 * protocol in `@angnative/contract`). Mutations are batched per change
 * detection tick and flushed through the injected `onFlush` callback — a
 * single FFI/message-queue call to the Rust host.
 *
 * Unknown property/style writes THROW (spec §5.3) — a silent device failure
 * becomes a loud test failure.
 */

import {
  Renderer2,
  RendererFactory2,
  RenderDebugInfo,
  RendererStyleFlags2,
  RendererType2,
} from '@angular/core';
import type { ElementContract } from '@angnative/contract/elements';
import { ELEMENT_TAGS, getContract, isKnownElement } from '@angnative/contract/elements';
import type { StyleMap } from '@angnative/contract/style';
import { validateStyleMap, parseStyleValue } from '@angnative/contract/style';
import type { Command } from '@angnative/contract/protocol';
import { createBatch, encodeBatch } from '@angnative/contract/protocol';
import {
  NativeElement,
  NativeNodeType,
  NodeId,
  ElementRegistry,
} from './native-element';

/** Hook the renderer uses to flush a command batch to the backend. */
export interface FlushHandler {
  /** Called once per change detection tick with the encoded JSON batch. */
  onFlush(batchJson: string): void;
}

/** Options for constructing the AngNative renderer. */
export interface AngNativeRendererOptions {
  /** Element contract registry (validates tags/styles). Defaults to the built-in set. */
  contracts?: Map<string, ElementContract>;
  /** Flush handler — required to actually push mutations to a backend. */
  flush?: FlushHandler;
}

/**
 * Tag → Rust NodeType mapping. Mirrors `NodeType::from_tag` in
 * `crates/angnative-core/src/node.rs`, canonicalized to the `na-*` prefix.
 */
export function tagToNodeType(tag: string): NativeNodeType {
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

/**
 * AngNative's custom `Renderer2`. Translates Angular template operations
 * into native view mutations and batches them for flush.
 */
export class AngNativeRenderer implements Renderer2 {
  private readonly registry: ElementRegistry;
  private readonly flush: FlushHandler | null;
  private readonly mutations: Command[] = [];
  private frameId = 0;
  private flushed = false;

  constructor(private readonly options: AngNativeRendererOptions = {}) {
    this.registry = new ElementRegistry();
    this.flush = options.flush ?? null;
  }

  /* ---------- lifecycle ---------- */

  destroy(): void {
    this.mutations.length = 0;
    this.frameId = 0;
    this.flushed = false;
  }

  /* ---------- creation ---------- */

  createElement(
    parent: any,
    name: string,
    _debugInfo?: RenderDebugInfo,
  ): any {
    if (!isKnownElement(name)) {
      throw new Error(
        `Unknown element "${name}". Use a canonical na-* tag (e.g. na-view, na-text).`,
      );
    }

    const contract = getContract(name)!;
    const nodeType = tagToNodeType(name);
    const id = this.registry.allocateId();

    const element = new NativeElement(id, name, nodeType);
    element.applyDefaults(contract.defaultStyles);
    this.registry.register(element);

    // Parent linkage.
    let parentId: NodeId | null = null;
    if (parent && typeof parent === 'object' && 'id' in parent) {
      parentId = (parent as NativeElement).id;
      const parentEl = this.registry.get(parentId);
      parentEl.addChild(id);
      element.parentId = parentId;
    }

    this.recordMutation({
      kind: 'CreateNode',
      id,
      nodeType,
    });

    if (parentId !== null) {
      this.recordMutation({
        kind: 'InsertChild',
        parentId,
        childId: id,
        index: this.registry.get(parentId).children.length - 1,
      });
    }

    this.maybeFlush();
    return element;
  }

  createComment(value: string, _debugInfo?: RenderDebugInfo): any {
    // Comments are not native nodes; return a lightweight marker node.
    const id = this.registry.allocateId();
    const element = new NativeElement(id, '#comment', 'Custom');
    element.setTextContent(value);
    this.registry.register(element);
    this.recordMutation({
      kind: 'CreateNode',
      id,
      nodeType: 'Custom',
    });
    this.recordMutation({ kind: 'SetTextContent', id, text: value });
    this.maybeFlush();
    return element;
  }

  createText(value: string, _debugInfo?: RenderDebugInfo): any {
    const id = this.registry.allocateId();
    const element = new NativeElement(id, '#text', 'Text');
    element.setTextContent(value);
    this.registry.register(element);
    this.recordMutation({
      kind: 'CreateNode',
      id,
      nodeType: 'Text',
    });
    this.recordMutation({ kind: 'SetTextContent', id, text: value });
    this.maybeFlush();
    return element;
  }

  /* ---------- tree mutation ---------- */

  appendChild(parent: any, newChild: any): void {
    const parentEl = this.asElement(parent);
    const childEl = this.asElement(newChild);

    // Detach from previous parent if re-parenting.
    if (childEl.parentId !== null && childEl.parentId !== parentEl.id) {
      const oldParent = this.registry.get(childEl.parentId);
      oldParent.removeChildId(childEl.id);
      this.recordMutation({
        kind: 'RemoveChild',
        parentId: childEl.parentId,
        childId: childEl.id,
      });
    }

    childEl.parentId = parentEl.id;
    parentEl.addChild(childEl.id);

    this.recordMutation({
      kind: 'InsertChild',
      parentId: parentEl.id,
      childId: childEl.id,
      index: parentEl.children.length - 1,
    });

    this.maybeFlush();
  }

  insertBefore(parent: any, newChild: any, refChild: any): void {
    const parentEl = this.asElement(parent);
    const childEl = this.asElement(newChild);
    const refEl = this.asElement(refChild);

    if (childEl.parentId !== null && childEl.parentId !== parentEl.id) {
      const oldParent = this.registry.get(childEl.parentId);
      oldParent.removeChildId(childEl.id);
      this.recordMutation({
        kind: 'RemoveChild',
        parentId: childEl.parentId,
        childId: childEl.id,
      });
    }

    childEl.parentId = parentEl.id;
    const idx = parentEl.children.indexOf(refEl.id);
    if (idx < 0) {
      parentEl.addChild(childEl.id);
    } else {
      parentEl.children.splice(idx, 0, childEl.id);
    }

    this.recordMutation({
      kind: 'InsertChild',
      parentId: parentEl.id,
      childId: childEl.id,
      index: idx < 0 ? parentEl.children.length - 1 : idx,
    });

    this.maybeFlush();
  }

  removeChild(parent: any, oldChild: any): void {
    const parentEl = this.asElement(parent);
    const childEl = this.asElement(oldChild);

    if (childEl.parentId !== parentEl.id) {
      throw new Error(
        `Element ${childEl.id} is not a child of element ${parentEl.id}`,
      );
    }

    childEl.parentId = null;
    parentEl.removeChildId(childEl.id);

    this.recordMutation({
      kind: 'RemoveChild',
      parentId: parentEl.id,
      childId: childEl.id,
    });

    this.maybeFlush();
  }

  selectRootElement(
    selectorOrNode: string | any,
    preserveContent = false,
  ): any {
    // No DOM in native mode. Treat the passed node as the root, or create a
    // default root na-view when given a selector string.
    if (typeof selectorOrNode === 'string') {
      return this.createElement(null, selectorOrNode);
    }
    return selectorOrNode;
  }

  parentNode(node: any): any {
    const el = this.asElement(node);
    if (el.parentId === null) return null;
    return this.registry.get(el.parentId);
  }

  nextSibling(node: any): any {
    const el = this.asElement(node);
    if (el.parentId === null) return null;
    const parent = this.registry.get(el.parentId);
    const idx = parent.children.indexOf(el.id);
    if (idx < 0 || idx === parent.children.length - 1) return null;
    return this.registry.get(parent.children[idx + 1]);
  }

  /* ---------- attributes / classes / styles ---------- */

  setAttribute(el: any, name: string, value: string, _namespace?: string): void {
    const element = this.asElement(el);
    element.setProperty(`attr:${name}`, value);
    this.recordMutation({
      kind: 'SetProperty',
      id: element.id,
      key: `attr:${name}`,
      value,
    });
    this.maybeFlush();
  }

  removeAttribute(el: any, name: string, _namespace?: string): void {
    const element = this.asElement(el);
    element.removeProperty(`attr:${name}`);
    this.recordMutation({
      kind: 'SetProperty',
      id: element.id,
      key: `attr:${name}`,
      value: null,
    });
    this.maybeFlush();
  }

  addClass(el: any, name: string): void {
    const element = this.asElement(el);
    const current = (element.properties.get('class') as string | undefined) ?? '';
    const classes = current ? current.split(/\s+/) : [];
    if (!classes.includes(name)) {
      classes.push(name);
      element.setProperty('class', classes.join(' '));
      this.recordMutation({
        kind: 'SetProperty',
        id: element.id,
        key: 'class',
        value: classes.join(' '),
      });
      this.maybeFlush();
    }
  }

  removeClass(el: any, name: string): void {
    const element = this.asElement(el);
    const current = (element.properties.get('class') as string | undefined) ?? '';
    const classes = current ? current.split(/\s+/) : [];
    const filtered = classes.filter((c) => c !== name);
    element.setProperty('class', filtered.join(' '));
    this.recordMutation({
      kind: 'SetProperty',
      id: element.id,
      key: 'class',
      value: filtered.join(' '),
    });
    this.maybeFlush();
  }

  setStyle(
    el: any,
    style: string,
    value: any,
    _flags?: RendererStyleFlags2,
  ): void {
    const element = this.asElement(el);
    // Validate + canonicalize against the style table (§5.3: unknown → throw).
    const canonical = parseStyleValue(style, String(value ?? ''));
    element.setStyle(style, canonical);
    this.recordMutation({
      kind: 'SetStyle',
      id: element.id,
      key: style,
      value: canonical,
    });
    this.maybeFlush();
  }

  removeStyle(el: any, style: string, _flags?: RendererStyleFlags2): void {
    const element = this.asElement(el);
    element.removeStyle(style);
    this.recordMutation({
      kind: 'SetStyle',
      id: element.id,
      key: style,
      value: '',
    });
    this.maybeFlush();
  }

  /* ---------- properties / text ---------- */

  setProperty(el: any, name: string, value: any): void {
    const element = this.asElement(el);
    element.setProperty(name, value);
    this.recordMutation({
      kind: 'SetProperty',
      id: element.id,
      key: name,
      value,
    });
    this.maybeFlush();
  }

  setValue(node: any, value: string): void {
    const element = this.asElement(node);
    element.setTextContent(value);
    this.recordMutation({
      kind: 'SetTextContent',
      id: element.id,
      text: value,
    });
    this.maybeFlush();
  }

  /* ---------- events ---------- */

  listen(
    target: 'window' | 'document' | 'body' | any,
    eventName: string,
    callback: (event: any) => boolean | void,
  ): () => void {
    let element: NativeElement | null = null;
    if (target === 'window' || target === 'document' || target === 'body') {
      // Global listeners are recorded against a synthetic root (id 0).
      element = null;
    } else {
      element = this.asElement(target);
    }

    const listenerId = this.registry.allocateId();
    this.recordMutation({
      kind: 'SetProperty',
      id: element ? element.id : 0,
      key: `listener:${eventName}:${listenerId}`,
      value: { eventName, callback },
    });

    this.maybeFlush();

    return () => {
      // Remove the listener registration.
      if (element) {
        element.removeProperty(`listener:${eventName}:${listenerId}`);
      }
      this.recordMutation({
        kind: 'SetProperty',
        id: element ? element.id : 0,
        key: `listener:${eventName}:${listenerId}`,
        value: null,
      });
      this.maybeFlush();
    };
  }

  /* ---------- internals ---------- */

  private asElement(node: any): NativeElement {
    if (node instanceof NativeElement) return node;
    if (node && typeof node === 'object' && 'id' in node) {
      return this.registry.get((node as NativeElement).id);
    }
    throw new Error(`Not a native element: ${JSON.stringify(node)}`);
  }

  private recordMutation(mutation: Command): void {
    this.mutations.push(mutation);
  }

  /** Flush the accumulated batch if a flush handler is attached. */
  private maybeFlush(): void {
    if (!this.flush) return;
    if (this.flushed) return; // already flushed this tick
    this.flushed = true;
    const batch = createBatch(this.frameId++, Date.now(), this.mutations);
    const json = encodeBatch(batch);
    this.flush.onFlush(json);
    this.mutations.length = 0;
    this.flushed = false;
  }

  /** Manually flush the current batch (call at end of change detection tick). */
  flushBatch(): void {
    if (this.mutations.length === 0 || !this.flush) return;
    const batch = createBatch(this.frameId++, Date.now(), this.mutations);
    const json = encodeBatch(batch);
    this.flush.onFlush(json);
    this.mutations.length = 0;
    this.flushed = false;
  }

  /** Expose the registry for testing / backends. */
  getRegistry(): ElementRegistry {
    return this.registry;
  }

  /** Pending (unflushed) mutation count. */
  pendingCount(): number {
    return this.mutations.length;
  }
}

/**
 * AngNative's `RendererFactory2`. Angular calls `createRenderer` to obtain
 * a renderer instance per renderer type. We return a shared
 * `AngNativeRenderer` (hosted renderer is a singleton per app).
 */
export class AngNativeRendererFactory2 implements RendererFactory2 {
  private readonly renderer: AngNativeRenderer;

  constructor(options: AngNativeRendererOptions = {}) {
    this.renderer = new AngNativeRenderer(options);
  }

  createRenderer(
    hostElement: any,
    _type: RendererType2 | null,
  ): Renderer2 {
    return this.renderer;
  }

  begin(): void {
    // No-op: native rendering batches per tick, not per element.
  }

  end(): void {
    // Flush any pending mutations at the end of the change detection tick.
    this.renderer.flushBatch();
  }

  /** The underlying renderer (useful for tests / backends). */
  getRenderer(): AngNativeRenderer {
    return this.renderer;
  }
}

/**
 * Bootstrap helper: create an AngNativeRendererFactory2 wired to a flush
 * handler that delivers batches to the Rust host (or fake backend).
 *
 * Example:
 *   const factory = bootstrapNativeAngular((batchJson) => {
 *     rustHost.dispatch(batchJson); // FFI → angnative-host-core
 *   });
 *   const renderer = factory.createRenderer(null, null);
 *
 * Pass the factory to Angular's `renderComponent` / `bootstrapApplication`
 * via a custom `RendererFactory2` provider.
 */
export function bootstrapNativeAngular(
  flushHandler: FlushHandler,
  options: AngNativeRendererOptions = {},
): AngNativeRendererFactory2 {
  return new AngNativeRendererFactory2({
    ...options,
    flush: flushHandler,
  });
}
