/**
 * renderer.ts — web `RendererFactory2` for AngNative.
 *
 * Reuses @angnative/core's mutation encoding verbatim: the renderer records
 * the same `Command` batches a native app would flush to Rust. On `end()` the
 * batch is decoded and translated into `PlatformOp`s (ops.rs) which are
 * applied to the DOM by `DomBackend`.
 *
 * Two web-specific concerns are handled by `WebRenderer`:
 *  - `selectRootElement` adopts a real host element instead of inventing a
 *    detached native node, so Angular's root lands in the document.
 *  - `listen` attaches a real DOM listener immediately (ops have no `Listen`
 *    variant), while the core renderer still records the listener command so
 *    the batch stays byte-compatible with the native path.
 */

import {
  InjectionToken,
  RendererFactory2,
  type Provider,
  type Renderer2,
  type RendererType2,
} from '@angular/core';
import {
  AngNativeRenderer,
  AngNativeRendererFactory2,
  NativeElement,
  type AngNativeRendererOptions,
} from '@angnative/core';
import {
  decodeBatch,
  type Command,
  type JsonValue,
} from '@angnative/contract/protocol';
import { isKnownElement } from '@angnative/contract/elements';
import { DomBackend, type PlatformOp } from './dom-backend';

/**
 * Translate a core `CommandBatch` into the ordered op stream the DOM backend
 * consumes. Consecutive style/property writes for one node are coalesced into
 * a single `SetProps` op. Listener registrations are intentionally skipped —
 * they are attached directly by `WebRenderer.listen`.
 */
export function commandsToPlatformOps(commands: readonly Command[]): PlatformOp[] {
  const ops: PlatformOp[] = [];
  const pending = new Map<number, { styles: Record<string, string>; properties: Record<string, JsonValue> }>();

  const pendingFor = (id: number) => {
    let entry = pending.get(id);
    if (!entry) {
      entry = { styles: {}, properties: {} };
      pending.set(id, entry);
    }
    return entry;
  };

  const drain = () => {
    for (const [node_id, props] of pending) {
      ops.push({ kind: 'SetProps', node_id, props });
    }
    pending.clear();
  };

  for (const command of commands) {
    switch (command.kind) {
      case 'CreateNode':
        drain();
        ops.push({ kind: 'Create', node_id: command.id, view_type: command.nodeType });
        break;
      case 'RemoveNode':
        drain();
        ops.push({ kind: 'Destroy', node_id: command.id });
        break;
      case 'InsertChild':
        drain();
        ops.push({
          kind: 'Insert',
          parent_id: command.parentId,
          child_id: command.childId,
          index: command.index,
        });
        break;
      case 'RemoveChild':
        drain();
        ops.push({
          kind: 'Remove',
          parent_id: command.parentId,
          child_id: command.childId,
        });
        break;
      case 'SetTextContent':
        drain();
        ops.push({ kind: 'SetText', node_id: command.id, text: command.text });
        break;
      case 'SetStyle':
        pendingFor(command.id).styles[command.key] = command.value;
        break;
      case 'SetProperty':
        if (!command.key.startsWith('listener:')) {
          pendingFor(command.id).properties[command.key] = command.value;
        }
        break;
    }
  }
  drain();
  return ops;
}

/** A core renderer that also keeps a real DOM node registered for every id. */
export class WebRenderer extends AngNativeRenderer {
  constructor(
    private readonly backend: DomBackend,
    private readonly doc: Document,
    options: AngNativeRendererOptions,
  ) {
    super(options);
  }

  override createElement(name: string, namespace?: string | null): any;
  /** Legacy overload kept in sync with the core renderer. */
  override createElement(parent: any, name: string): any;
  override createElement(nameOrParent: any, nameOrNamespace?: any): any {
    const parent = typeof nameOrParent === 'string' ? null : nameOrParent;
    const name = typeof nameOrParent === 'string' ? nameOrParent : nameOrNamespace;

    if (typeof name !== 'string') {
      throw new TypeError('createElement requires an element name');
    }

    // Angular component selectors (e.g. <demo-counter>) are not na-* elements.
    // Core rejects them, but on the web they are ordinary custom DOM tags.
    if (!isKnownElement(name)) {
      const registry = this.getRegistry();
      const id = registry.allocateId();
      const element = new NativeElement(id, name, 'Custom');
      registry.register(element);
      if (parent && typeof parent === 'object' && 'id' in parent) {
        (parent as NativeElement).addChild?.(id);
        element.parentId = (parent as NativeElement).id;
      }
      this.registerDomNode(element);
      return element;
    }

    const element = super.createElement(nameOrParent, nameOrNamespace) as NativeElement;
    this.registerDomNode(element);
    return element;
  }

  override setStyle(el: any, style: string, value: any, flags?: any): void {
    // Angular may emit kebab-case ([style.flex-direction]); the shared style
    // table uses camelCase, so normalize before delegating to core.
    const camel = style.replace(/-([a-z])/g, (_, c: string) => c.toUpperCase());
    super.setStyle(el, camel, value, flags);
  }

  override createComment(value: string): any {
    const element = super.createComment(value) as NativeElement;
    this.registerDomNode(element);
    return element;
  }

  override createText(value: string): any {
    const element = super.createText(value) as NativeElement;
    this.registerDomNode(element);
    return element;
  }

  /**
   * Adopt the real host element for the root component. If no host exists in
   * the document we create one so bootstrapping can proceed.
   */
  override selectRootElement(selectorOrNode: string | any, preserveContent = false): any {
    if (typeof selectorOrNode !== 'string') {
      if (selectorOrNode instanceof NativeElement) this.registerDomNode(selectorOrNode);
      return selectorOrNode;
    }

    let host: Element | null = null;
    try {
      host = this.doc.querySelector(selectorOrNode);
    } catch {
      host = null;
    }
    if (!host) {
      host = this.doc.createElement(selectorOrNode);
      (this.doc.body ?? this.doc.documentElement).appendChild(host);
    }
    if (!preserveContent) host.textContent = '';

    const registry = this.getRegistry();
    const id = registry.allocateId();
    const element = new NativeElement(id, selectorOrNode, 'Custom');
    registry.register(element);
    this.backend.adoptRoot(id, host);
    return element;
  }

  /**
   * Angular's view detach passes a null parent for root-level nodes. The core
   * renderer expects a real parent, so resolve it from the shadow registry (or
   * destroy the node outright when it has no parent).
   */
  override removeChild(parent: any, oldChild: any): void {
    if (parent != null) {
      super.removeChild(parent, oldChild);
      return;
    }
    const element = oldChild as NativeElement;
    const parentId = element && typeof element === 'object' && 'parentId' in element
      ? element.parentId
      : null;
    if (parentId != null && this.getRegistry().has(parentId)) {
      super.removeChild(this.getRegistry().get(parentId), oldChild);
    } else {
      this.destroyNode?.(oldChild);
    }
  }

  override listen(
    target: 'window' | 'document' | 'body' | any,
    eventName: string,
    callback: (event: any) => boolean | void,
    options?: { capture?: boolean; once?: boolean; passive?: boolean },
  ): () => void {
    const unlistenCore = super.listen(target, eventName, callback, options);
    const id =
      target && typeof target === 'object' && 'id' in target
        ? (target as NativeElement).id
        : DomBackend.GLOBAL_NODE_ID;
    this.backend.addEventListener(id, eventName, callback, options);
    return () => {
      unlistenCore();
      this.backend.removeEventListener(id, eventName, callback);
    };
  }

  private registerDomNode(element: NativeElement): void {
    if (this.backend.hasNode(element.id)) return;
    const node =
      element.tag === '#text'
        ? this.doc.createTextNode('')
        : element.tag === '#comment'
          ? this.doc.createComment('')
          : this.doc.createElement(element.tag);
    if (node.nodeType === 1) {
      (node as Element).setAttribute('data-na-node-id', String(element.id));
    }
    this.backend.registerNode(element.id, node);
  }
}

export interface WebRendererOptions {
  /** DOM document to render into. Defaults to `globalThis.document`. */
  doc?: Document;
  /** Existing backend (useful for inspecting/testing the produced DOM). */
  backend?: DomBackend;
}

/**
 * Angular `RendererFactory2`. One shared renderer per app, mirroring
 * `AngNativeRendererFactory2`'s single-renderer design.
 */
export class WebRendererFactory2 implements RendererFactory2 {
  readonly document: Document;
  readonly backend: DomBackend;
  private readonly renderer: WebRenderer;

  constructor(options: WebRendererOptions = {}) {
    this.document = options.doc ?? globalThis.document;
    if (!this.document) {
      throw new Error('WebRendererFactory2 requires a DOM Document');
    }
    this.backend = options.backend ?? new DomBackend(this.document);
    if (!this.backend.hasNode(DomBackend.GLOBAL_NODE_ID)) {
      this.backend.registerNode(DomBackend.GLOBAL_NODE_ID, this.document as unknown as Node);
    }
    this.renderer = new WebRenderer(this.backend, this.document, {
      flush: { onFlush: (json) => this.flush(json) },
    });
  }

  createRenderer(_hostElement: any, _type: RendererType2 | null): Renderer2 {
    return this.renderer;
  }

  begin(): void {
    // Core renderer batches per tick; nothing to do at begin.
  }

  end(): void {
    this.renderer.flushBatch();
  }

  /** The underlying renderer (tests / advanced integrations). */
  getRenderer(): AngNativeRenderer {
    return this.renderer;
  }

  /** Flush a core command batch into the DOM backend. */
  flush(batchJson: string): void {
    const batch = decodeBatch(batchJson);
    this.backend.apply(commandsToPlatformOps(batch.commands));
  }
}

/** DI token for the web factory instance. */
export const WEB_RENDERER_FACTORY = new InjectionToken<WebRendererFactory2>(
  'ANGNATIVE_WEB_RENDERER_FACTORY',
);

/**
 * Providers that wire the web factory into Angular.
 *
 * `AngNativeRendererFactory2` is provided explicitly so the `na-*` primitives
 * from `@angnative/common` (which inject that token) work on the web without
 * requiring a separate native-platform provider.
 */
export function provideWebRendererFactory(options: WebRendererOptions = {}): Provider[] {
  return [
    {
      provide: WEB_RENDERER_FACTORY,
      useFactory: () => new WebRendererFactory2(options),
    },
    { provide: AngNativeRendererFactory2, useExisting: WEB_RENDERER_FACTORY },
    { provide: RendererFactory2, useExisting: WEB_RENDERER_FACTORY },
  ];
}
