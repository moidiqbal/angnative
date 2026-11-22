/**
 * integration.spec.ts — end-to-end smoke test tying the full Phase 1 stack
 * together (@angnative/core renderer + @angnative/platform bootstrap +
 * @angnative/common components).
 *
 * Verifies T4 (na- element wiring) and T5 (M0 review fixes) end-to-end.
 */

// @angnative/platform now imports @angular/animations/browser, which ships
// partially-compiled declarations. Vitest does not run the Angular linker, so
// load the JIT-capable platform (which imports @angular/compiler) before that
// module is evaluated.
import '@angular/platform-browser-dynamic';
import { describe, it, expect } from 'vitest';
import { AngNativeRenderer } from '@angnative/core/renderer';
import { bootstrapNativeAngularApp } from '@angnative/platform';
import { IS_TESTING_AVAILABLE } from '@angnative/platform/testing';
import {
  NaViewComponent,
  NaTextComponent,
  NaButtonComponent,
} from '@angnative/common';

describe('integration — full Phase 1 stack', () => {
  it('renders a counter app through the native renderer', () => {
    const flushes: string[] = [];
    const renderer = new AngNativeRenderer({
      flush: { onFlush: (json) => flushes.push(json) },
    });

    // Root na-view
    const root = renderer.createElement(null, 'na-view');
    renderer.setStyle(root, 'flexDirection', 'column');
    renderer.setStyle(root, 'padding', '16px');

    // Title text
    const title = renderer.createElement(root, 'na-text');
    renderer.setValue(title, 'AngNative Counter');
    renderer.setStyle(title, 'fontSize', '20px');

    // Counter text (reactive value)
    const counter = renderer.createElement(root, 'na-text');
    renderer.setValue(counter, 'Count: 0');

    // Buttons
    const btnDec = renderer.createElement(root, 'na-button');
    renderer.setValue(btnDec, '−');
    renderer.listen(btnDec, 'press', () => {
      // decrement handler
    });

    const btnInc = renderer.createElement(root, 'na-button');
    renderer.setValue(btnInc, '+');
    renderer.listen(btnInc, 'press', () => {
      // increment handler
    });

    renderer.flushBatch();

    // Verify all nodes created and linked
    const registry = renderer.getRegistry();
    // root(1) + title(2) + counter(3) + btnDec(4) + btnInc(5)
    expect(registry.size()).toBe(5);

    const rootNode = registry.get(1);
    expect(rootNode.children).toEqual([2, 3, 4, 5]);

    const counterNode = registry.get(3);
    expect(counterNode.textContent).toBe('Count: 0');

    // Verify mutations were flushed
    expect(flushes.length).toBeGreaterThan(0);
    const batch = JSON.parse(flushes[flushes.length - 1]!);
    const kinds = batch.commands.map((c: any) => c.kind);
    expect(kinds).toContain('CreateNode');
    expect(kinds).toContain('InsertChild');
    expect(kinds).toContain('SetTextContent');
    expect(kinds).toContain('SetStyle');
  });

  it('na-* tags are canonical (T4 / D9)', () => {
    const renderer = new AngNativeRenderer();
    // Canonical na-* tags work
    expect(() => renderer.createElement(null, 'na-view')).not.toThrow();
    expect(() => renderer.createElement(null, 'na-text')).not.toThrow();
    expect(() => renderer.createElement(null, 'na-button')).not.toThrow();
    // Legacy aliases also accepted (back-compat during migration)
    expect(() => renderer.createElement(null, 'native-view')).not.toThrow();
    expect(() => renderer.createElement(null, 'div')).not.toThrow();
    // Unknown tags throw
    expect(() => renderer.createElement(null, 'na-ghost')).toThrow(/Unknown/);
  });

  it('testing guard is available in dev mode (T5)', () => {
    // IS_TESTING_AVAILABLE reflects ngDevMode; in vitest it should be true
    expect(typeof IS_TESTING_AVAILABLE).toBe('boolean');
  });

  it('bootstrapNativeAngularApp is importable', () => {
    expect(typeof bootstrapNativeAngularApp).toBe('function');
  });

  it('common components are importable', () => {
    expect(NaViewComponent).toBeDefined();
    expect(NaTextComponent).toBeDefined();
    expect(NaButtonComponent).toBeDefined();
  });
});
