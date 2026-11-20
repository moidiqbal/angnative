/**
 * dom-backend.ts — applies Rust `PlatformOp`s (crates/angnative-host-view/src/ops.rs)
 * to a real DOM tree.
 *
 * The backend owns the authoritative `node_id -> DOM Node` map. Nodes are
 * created by `Create` ops, but the web renderer may pre-register a DOM node
 * (`registerNode`) so that event listeners can be attached before the command
 * batch is flushed. `Create` is therefore idempotent for a known id — this is
 * what lets the same op stream drive both a cold DOM and an already-live one.
 *
 * Nothing here depends on Angular: this is a pure, directly testable sink.
 */

import type { JsonValue } from '@angnative/contract/protocol';

/** TS mirror of Rust `ViewProps` (ops.rs). */
export interface ViewProps {
  readonly text?: string | null;
  readonly styles?: Readonly<Record<string, string>>;
  readonly properties?: Readonly<Record<string, JsonValue>>;
}

/** TS mirror of Rust `PlatformOp` (ops.rs). */
export type PlatformOp =
  | { readonly kind: 'Create'; readonly node_id: number; readonly view_type: string }
  | {
      readonly kind: 'SetFrame';
      readonly node_id: number;
      readonly x: number;
      readonly y: number;
      readonly width: number;
      readonly height: number;
    }
  | { readonly kind: 'SetProps'; readonly node_id: number; readonly props: ViewProps }
  | { readonly kind: 'SetText'; readonly node_id: number; readonly text: string }
  | { readonly kind: 'Append'; readonly parent_id: number; readonly child_id: number }
  | {
      readonly kind: 'Insert';
      readonly parent_id: number;
      readonly child_id: number;
      readonly index: number;
    }
  | { readonly kind: 'Remove'; readonly parent_id: number; readonly child_id: number }
  | { readonly kind: 'Destroy'; readonly node_id: number };

/** Native view types → concrete DOM tags (see `NodeType::from_tag` in Rust). */
export const VIEW_TYPE_TAGS: Readonly<Record<string, string>> = {
  View: 'na-view',
  Text: 'na-text',
  Button: 'na-button',
  Image: 'na-image',
  ScrollView: 'na-scroll-view',
  TextInput: 'na-text-input',
  Custom: 'na-view',
};

const TEXT_VIEW_TYPES = new Set(['#text', 'TextNode', 'text']);
const COMMENT_VIEW_TYPES = new Set(['#comment', 'Comment']);

/** Convert a canonical camelCase style key to the DOM CSS property name. */
export function styleKeyToCss(key: string): string {
  return key.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`);
}

interface ListenerRecord {
  readonly target: EventTarget;
  readonly type: string;
  readonly fn: EventListener;
  readonly options?: boolean | AddEventListenerOptions;
}

export class DomBackend {
  /** Node id 0 is the synthetic target Angular uses for global listeners. */
  static readonly GLOBAL_NODE_ID = 0;

  private readonly nodes = new Map<number, Node>();
  private readonly listeners = new Map<number, ListenerRecord[]>();

  constructor(private readonly doc: Document = globalThis.document) {
    if (!doc) {
      throw new Error('DomBackend requires a DOM Document');
    }
    this.nodes.set(DomBackend.GLOBAL_NODE_ID, doc as unknown as Node);
  }

  /** Register (or replace) the DOM node backing a node id. */
  registerNode(id: number, node: Node): void {
    this.nodes.set(id, node);
  }

  /** Adopt an already-existing DOM element as the root for a node id. */
  adoptRoot(id: number, element: Element): void {
    this.registerNode(id, element);
  }

  hasNode(id: number): boolean {
    return this.nodes.has(id);
  }

  getNode(id: number): Node | undefined {
    return this.nodes.get(id);
  }

  /** Apply an ordered commit of platform operations. */
  apply(ops: readonly PlatformOp[]): void {
    for (const op of ops) {
      this.applyOp(op);
    }
  }

  /** Attach a DOM listener for a renderer callback (events are not ops). */
  addEventListener(
    id: number,
    type: string,
    fn: (event: any) => boolean | void,
    options?: { capture?: boolean; once?: boolean; passive?: boolean },
  ): void {
    const target = this.eventTarget(id);
    const listener: EventListener = (event) => {
      fn(event);
    };
    target.addEventListener(type, listener, options);
    const existing = this.listeners.get(id) ?? [];
    existing.push({ target, type, fn: listener, options });
    this.listeners.set(id, existing);
  }

  /** Detach a previously attached DOM listener. */
  removeEventListener(id: number, type: string, fn: (event: any) => boolean | void): void {
    const records = this.listeners.get(id);
    if (!records) return;
    const kept: ListenerRecord[] = [];
    for (const record of records) {
      if (record.type === type) {
        record.target.removeEventListener(type, record.fn, record.options);
      } else {
        kept.push(record);
      }
    }
    if (kept.length === 0) this.listeners.delete(id);
    else this.listeners.set(id, kept);
    // The `fn` parameter is intentionally accepted for Renderer2 symmetry;
    // listeners are tracked per (node, event type) by the renderer callback id.
    void fn;
  }

  /** Remove every listener registered for a node (called on Destroy). */
  removeListenersFor(id: number): void {
    const records = this.listeners.get(id) ?? [];
    for (const record of records) {
      record.target.removeEventListener(record.type, record.fn, record.options);
    }
    this.listeners.delete(id);
  }

  private eventTarget(id: number): EventTarget {
    if (id === DomBackend.GLOBAL_NODE_ID) return this.doc;
    const node = this.requireNode(id);
    return node as unknown as EventTarget;
  }

  private requireNode(id: number): Node {
    const node = this.nodes.get(id);
    if (!node) throw new Error(`DomBackend: no DOM node registered for id ${id}`);
    return node;
  }

  private requireElement(id: number): Element {
    const node = this.requireNode(id);
    if (node.nodeType !== 1) {
      throw new Error(`DomBackend: node ${id} is not an element`);
    }
    return node as Element;
  }

  private create(op: Extract<PlatformOp, { kind: 'Create' }>): void {
    if (this.nodes.has(op.node_id)) return; // pre-registered or adopted

    let node: Node;
    if (TEXT_VIEW_TYPES.has(op.view_type)) {
      node = this.doc.createTextNode('');
    } else if (COMMENT_VIEW_TYPES.has(op.view_type)) {
      node = this.doc.createComment('');
    } else {
      const tag = VIEW_TYPE_TAGS[op.view_type] ?? op.view_type ?? 'na-view';
      node = this.doc.createElement(tag);
      (node as Element).setAttribute('data-na-node-id', String(op.node_id));
    }
    this.nodes.set(op.node_id, node);
  }

  private setProps(op: Extract<PlatformOp, { kind: 'SetProps' }>): void {
    const node = this.requireNode(op.node_id);
    if (node.nodeType !== 1) {
      // Text nodes only carry text content.
      if (op.props.text != null) this.setText({ kind: 'SetText', node_id: op.node_id, text: String(op.props.text) });
      return;
    }

    const el = node as Element;
    const styles = op.props.styles ?? {};
    const css = (el as unknown as { style?: CSSStyleDeclaration }).style;
    for (const [key, value] of Object.entries(styles)) {
      if (!css) break;
      const cssKey = styleKeyToCss(key);
      if (value === '' || value == null) css.removeProperty(cssKey);
      else css.setProperty(cssKey, String(value));
    }

    const props = op.props.properties ?? {};
    for (const [key, value] of Object.entries(props)) {
      if (key.startsWith('listener:')) continue; // handled by the renderer
      if (value == null) {
        el.removeAttribute(this.attributeName(key));
        continue;
      }
      if (key === 'class' || key === 'className') {
        el.setAttribute('class', String(value));
      } else if (key === 'value' && 'value' in el) {
        (el as unknown as { value: string }).value = String(value);
        el.setAttribute('value', String(value));
      } else if (key === 'disabled' || key === 'checked') {
        (el as unknown as Record<string, boolean>)[key] = Boolean(value);
        if (value) el.setAttribute(key, '');
        else el.removeAttribute(key);
      } else {
        el.setAttribute(this.attributeName(key), String(value));
      }
    }

    if (op.props.text != null) {
      this.setText({ kind: 'SetText', node_id: op.node_id, text: String(op.props.text) });
    }
  }

  private attributeName(key: string): string {
    return key.startsWith('attr:') ? key.slice(5) : key;
  }

  private setText(op: Extract<PlatformOp, { kind: 'SetText' }>): void {
    const node = this.requireNode(op.node_id);
    if (node.nodeType === 3) {
      node.nodeValue = op.text;
    } else {
      node.textContent = op.text;
    }
  }

  private setFrame(op: Extract<PlatformOp, { kind: 'SetFrame' }>): void {
    const el = this.requireElement(op.node_id) as unknown as HTMLElement;
    el.style.position = 'absolute';
    el.style.left = `${op.x}px`;
    el.style.top = `${op.y}px`;
    el.style.width = `${op.width}px`;
    el.style.height = `${op.height}px`;
  }

  private insert(
    parentId: number,
    childId: number,
    index: number | null,
  ): void {
    const parent = this.requireNode(parentId);
    const child = this.requireNode(childId);
    if (index === null) {
      parent.appendChild(child);
      return;
    }
    const siblings = parent.childNodes;
    const ref = index >= 0 && index < siblings.length ? siblings[index]! : null;
    parent.insertBefore(child, ref);
  }

  private applyOp(op: PlatformOp): void {
    switch (op.kind) {
      case 'Create':
        this.create(op);
        break;
      case 'SetProps':
        this.setProps(op);
        break;
      case 'SetText':
        this.setText(op);
        break;
      case 'Append':
        this.insert(op.parent_id, op.child_id, null);
        break;
      case 'Insert':
        this.insert(op.parent_id, op.child_id, op.index);
        break;
      case 'Remove': {
        const parent = this.requireNode(op.parent_id);
        const child = this.requireNode(op.child_id);
        if (child.parentNode === parent) parent.removeChild(child);
        break;
      }
      case 'SetFrame':
        this.setFrame(op);
        break;
      case 'Destroy': {
        const node = this.requireNode(op.node_id);
        node.parentNode?.removeChild(node);
        this.removeListenersFor(op.node_id);
        this.nodes.delete(op.node_id);
        break;
      }
    }
  }
}
