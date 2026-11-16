/**
 * @angnative/core — custom Angular `RendererFactory2` / `Renderer2` for native
 * (non-DOM) rendering (Architecture §3.1).
 *
 * Translates Angular template operations into native view mutations that are
 * batched and flushed to the Rust host via the versioned command protocol
 * (`@angnative/contract/protocol`).
 */

export {
  AngNativeRenderer,
  AngNativeRendererFactory2,
  bootstrapNativeAngular,
  type AngNativeRendererOptions,
  type FlushHandler,
  tagToNodeType,
} from './renderer';

export {
  NativeElement,
  ElementRegistry,
  type NodeId,
  type NativeNodeType,
} from './native-element';
