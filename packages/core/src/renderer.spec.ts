import { describe, it, expect } from 'vitest';
import {
  AngNativeRenderer,
  AngNativeRendererFactory2,
  bootstrapNativeAngular,
  tagToNodeType,
} from './renderer';
import { NativeElement, ElementRegistry } from './native-element';

describe('tagToNodeType (mirrors Rust NodeType::from_tag)', () => {
  it('maps canonical na-* tags', () => {
    expect(tagToNodeType('na-view')).toBe('View');
    expect(tagToNodeType('na-text')).toBe('Text');
    expect(tagToNodeType('na-button')).toBe('Button');
    expect(tagToNodeType('na-image')).toBe('Image');
    expect(tagToNodeType('na-scroll-view')).toBe('ScrollView');
    expect(tagToNodeType('na-text-input')).toBe('TextInput');
  });

  it('accepts legacy aliases', () => {
    expect(tagToNodeType('native-view')).toBe('View');
    expect(tagToNodeType('div')).toBe('View');
    expect(tagToNodeType('span')).toBe('Text');
    expect(tagToNodeType('img')).toBe('Image');
  });

  it('defaults unknown to Custom', () => {
    expect(tagToNodeType('na-foo')).toBe('Custom');
    expect(tagToNodeType('custom-tag')).toBe('Custom');
  });
});

describe('AngNativeRenderer mutation batching', () => {
  let flushes: string[] = [];
  let renderer: AngNativeRenderer;

  beforeEach(() => {
    flushes = [];
    renderer = new AngNativeRenderer({
      flush: { onFlush: (json) => flushes.push(json) },
    });
  });

  function lastBatch(): any {
    return JSON.parse(flushes[flushes.length - 1]);
  }

  it('emits CreateNode + InsertChild on createElement with parent', () => {
    const root = renderer.createElement(null, 'na-view');
    const child = renderer.createElement(root, 'na-text');
    renderer.setValue(child, 'hello');

    const batch = lastBatch();
    const kinds = batch.commands.map((c: any) => c.kind);
    expect(kinds).toContain('CreateNode');
    expect(kinds).toContain('InsertChild');
    expect(kinds).toContain('SetTextContent');

    const create = batch.commands.find((c: any) => c.kind === 'CreateNode' && c.id === child.id);
    expect(create.nodeType).toBe('Text');
  });

  it('rejects unknown elements loudly (§5.3 analogue)', () => {
    expect(() => renderer.createElement(null, 'na-ghost')).toThrow(/Unknown/);
    expect(() => renderer.createElement(null, 'div')).not.toThrow(); // legacy alias allowed
  });

  it('setStyle validates against the style table', () => {
    const el = renderer.createElement(null, 'na-view');
    renderer.setStyle(el, 'flexDirection', 'row');
    renderer.setStyle(el, 'width', '200');

    const setStyle = lastBatch().commands.find(
      (c: any) => c.kind === 'SetStyle' && c.key === 'flexDirection',
    );
    expect(setStyle.value).toBe('row');

    const setWidth = lastBatch().commands.find(
      (c: any) => c.kind === 'SetStyle' && c.key === 'width',
    );
    expect(setWidth.value).toBe('200px');
  });

  it('setStyle throws on unknown property', () => {
    const el = renderer.createElement(null, 'na-view');
    expect(() => renderer.setStyle(el, 'border-box', '1px')).toThrow(/Unknown/);
  });

  it('setProperty records arbitrary values', () => {
    const el = renderer.createElement(null, 'na-view');
    renderer.setProperty(el, 'disabled', true);
    const setProp = lastBatch().commands.find(
      (c: any) => c.kind === 'SetProperty' && c.key === 'disabled',
    );
    expect(setProp.value).toBe(true);
  });

  it('appendChild / removeChild record mutations', () => {
    const parent = renderer.createElement(null, 'na-view');
    const child = renderer.createElement(null, 'na-view');
    renderer.appendChild(parent, child);
    renderer.removeChild(parent, child);

    const kinds = lastBatch().commands.map((c: any) => c.kind);
    expect(kinds).toContain('InsertChild');
    expect(kinds).toContain('RemoveChild');
  });

  it('flushBatch emits a single batch then clears', () => {
    const el = renderer.createElement(null, 'na-view');
    renderer.setStyle(el, 'opacity', '0.5');
    renderer.flushBatch();
    expect(flushes.length).toBe(1);
    expect(renderer.pendingCount()).toBe(0);
    renderer.setStyle(el, 'opacity', '1');
    expect(renderer.pendingCount()).toBe(1);
    renderer.flushBatch();
    expect(flushes.length).toBe(2);
  });

  it('no flush handler => no throws, mutations just accumulate', () => {
    const noFlush = new AngNativeRenderer({});
    const el = noFlush.createElement(null, 'na-view');
    noFlush.setStyle(el, 'width', '100');
    expect(noFlush.pendingCount()).toBe(1);
    expect(() => noFlush.flushBatch()).not.toThrow();
  });
});

describe('AngNativeRendererFactory2', () => {
  it('returns a shared renderer and flushes on end()', () => {
    const flushes: string[] = [];
    const factory = new AngNativeRendererFactory2({
      flush: { onFlush: (json) => flushes.push(json) },
    });
    const r1 = factory.createRenderer(null, null);
    const r2 = factory.createRenderer(null, null);
    expect(r1).toBe(r2); // singleton

    const el = r1.createElement(null, 'na-view');
    r1.setStyle(el, 'height', '50');
    factory.end();
    expect(flushes.length).toBe(1);
  });

  it('bootstrapNativeAngular wires a flush handler', () => {
    const flushes: string[] = [];
    const factory = bootstrapNativeAngular({
      onFlush: (json) => flushes.push(json),
    });
    const renderer = factory.createRenderer(null, null);
    const el = renderer.createElement(null, 'na-text');
    renderer.setValue(el, 'hi');
    factory.end();
    expect(flushes.length).toBe(1);
    const batch = JSON.parse(flushes[0]);
    const setText = batch.commands.find((c: any) => c.kind === 'SetTextContent');
    expect(setText.text).toBe('hi');
  });

  it('NativeElement applyDefaults + registry', () => {
    const registry = new ElementRegistry();
    const el = new NativeElement(registry.allocateId(), 'na-view', 'View');
    registry.register(el);
    expect(registry.size()).toBe(1);
    expect(registry.get(el.id)).toBe(el);
    registry.remove(el.id);
    expect(registry.size()).toBe(0);
  });
});
