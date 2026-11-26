import { beforeEach, describe, it, expect } from 'vitest';
import {
  AngNativeRenderer,
  AngNativeRendererFactory2,
  bootstrapNativeAngular,
  canonicalElementTag,
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
    expect(tagToNodeType('na-pressable')).toBe('Button');
  });

  it('uses exact tagged custom wire types for the custom primitives', () => {
    expect(tagToNodeType('na-list')).toBe('Custom:na-list');
    expect(tagToNodeType('na-switch')).toBe('Custom:na-switch');
    expect(tagToNodeType('na-activity-indicator')).toBe('Custom:na-activity-indicator');
    expect(tagToNodeType('na-candlestick')).toBe('Custom:na-candlestick');
  });

  it('accepts legacy aliases', () => {
    expect(tagToNodeType('native-view')).toBe('View');
    expect(tagToNodeType('div')).toBe('View');
    expect(tagToNodeType('span')).toBe('Text');
    expect(tagToNodeType('img')).toBe('Image');
  });

  it('defaults unknown to untagged Custom', () => {
    expect(tagToNodeType('na-foo')).toBe('Custom');
    expect(tagToNodeType('custom-tag')).toBe('Custom');
  });
});

describe('canonicalElementTag', () => {
  it('maps HTML input to the native text input primitive', () => {
    expect(canonicalElementTag('input')).toBe('na-text-input');
  });

  it('keeps canonical custom primitive tags intact', () => {
    expect(canonicalElementTag('na-list')).toBe('na-list');
    expect(canonicalElementTag('na-switch')).toBe('na-switch');
    expect(canonicalElementTag('NA-ACTIVITY-INDICATOR')).toBe('na-activity-indicator');
    expect(canonicalElementTag('NA-CANDLESTICK')).toBe('na-candlestick');
    expect(canonicalElementTag('na-pressable')).toBe('na-pressable');
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
    renderer.flushBatch();
    return JSON.parse(flushes[flushes.length - 1]!);
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

  it('emit tagged CreateNode ops for canonical custom primitives', () => {
    for (const [tag, nodeType] of [
      ['na-list', 'Custom:na-list'],
      ['na-switch', 'Custom:na-switch'],
      ['na-activity-indicator', 'Custom:na-activity-indicator'],
      ['na-candlestick', 'Custom:na-candlestick'],
    ] as const) {
      const el = renderer.createElement(null, tag);
      expect(el.nativeNodeType).toBe(nodeType);

      const create = lastBatch().commands.find(
        (c: any) => c.kind === 'CreateNode' && c.id === el.id,
      );
      expect(create.nodeType).toBe(nodeType);
    }
  });

  it('rejects unknown elements loudly (§5.3 analogue)', () => {
    expect(() => renderer.createElement(null, 'na-ghost')).toThrow(/Unknown/);
    expect(() => renderer.createElement(null, 'div')).not.toThrow(); // normalized legacy alias
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

  it('canonicalizes kebab style keys before recording SetStyle', () => {
    const el = renderer.createElement(null, 'na-view');
    renderer.setStyle(el, 'flex-direction', 'row');
    renderer.setStyle(el, 'font-size', '24px');
    renderer.setStyle(el, 'margin-top', '12px');

    const keys = lastBatch().commands
      .filter((command: any) => command.kind === 'SetStyle')
      .map((command: any) => command.key);
    expect(keys).toContain('flexDirection');
    expect(keys).toContain('fontSize');
    expect(keys).toContain('marginTop');
    expect(keys).not.toContain('flex-direction');
    expect((el as any).styles.get('flexDirection')).toBe('row');
  });

  it('animate() emits interpolated native style mutations and a finish event', async () => {
    const el = renderer.createElement(null, 'na-view');
    let finished = 0;
    const player = (el as any).animate(
      [{ offset: 0, opacity: 0 }, { offset: 1, opacity: 1 }],
      { duration: 100 },
    ) as {
      addEventListener(type: string, listener: () => void): void;
      play(): unknown;
    };
    player.addEventListener('finish', () => { finished++; });
    player.play();

    const values = lastBatch().commands
      .filter((command: any) => command.kind === 'SetStyle' && command.key === 'opacity')
      .map((command: any) => command.value);
    expect(values).toEqual(['0', '0.5', '1']);

    await new Promise<void>((resolve) => queueMicrotask(resolve));
    expect(finished).toBe(1);
  });

  it('animate() with easing emits host-clock eased SetStyle mutations', () => {
    const globals = globalThis as unknown as Record<string, unknown>;
    const keys = ['na_nowMs', 'requestAnimationFrame', 'cancelAnimationFrame'] as const;
    const saved = new Map<string, unknown>(keys.map((key) => [key, globals[key]]));
    let now = 0;
    const frames: Array<(timestamp: number) => void> = [];
    globals['na_nowMs'] = () => now;
    globals['requestAnimationFrame'] = (callback: (timestamp: number) => void) => {
      frames.push(callback);
      return frames.length;
    };
    globals['cancelAnimationFrame'] = (id: number) => {
      if (id > 0 && id <= frames.length) frames[id - 1] = () => {};
    };

    try {
      const el = renderer.createElement(null, 'na-view');
      renderer.flushBatch();
      const player = (el as any).animate(
        [{ offset: 0, opacity: 0 }, { offset: 1, opacity: 1 }],
        { duration: 100, easing: 'ease-in' },
      ) as { play(): unknown };
      player.play();

      const opacityValues = () => {
        renderer.flushBatch();
        return flushes.flatMap((json) => {
          const batch = JSON.parse(json);
          return batch.commands
            .filter((command: any) => command.kind === 'SetStyle' && command.key === 'opacity')
            .map((command: any) => String(command.value));
        });
      };

      expect(opacityValues()).toEqual(['0']);
      now = 50;
      frames.shift()?.(0);
      expect(opacityValues()).toEqual(['0', '0.25']);
      now = 100;
      frames.shift()?.(0);
      expect(opacityValues()).toEqual(['0', '0.25', '1']);
    } finally {
      for (const [key, value] of saved) {
        if (value === undefined) delete globals[key];
        else globals[key] = value;
      }
    }
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

  it('registers JSON-safe event listeners and dispatches them by stable listener id', () => {
    const el = renderer.createElement(null, 'na-button');
    let received: unknown;
    const unlisten = renderer.listen(el, 'press', (event) => {
      received = event;
    });

    const batch = lastBatch();
    const listener = batch.commands.find(
      (command: any) => command.kind === 'SetProperty' && command.key.startsWith('listener:press:'),
    );
    expect(listener.value).toEqual({ eventName: 'press', listenerId: 1 });
    const event = { source: 'native' };
    renderer.dispatchEvent(listener.value.listenerId, event);
    expect(received).toBe(event);

    unlisten();
    expect(renderer.dispatchEvent(listener.value.listenerId, event)).toBeUndefined();
  });

  it('routes native events to listeners by node id and event name', () => {
    const el = renderer.createElement(null, 'na-button');
    const received: unknown[] = [];
    renderer.listen(el, 'press', (event) => {
      received.push(event);
    });

    const event = { source: 'native' };
    expect(renderer.dispatchNativeEvent(el.id, 'press', event)).toBe(true);
    expect(received).toEqual([event]);

    // A different event name or node must not fire the listener.
    expect(renderer.dispatchNativeEvent(el.id, 'release', event)).toBe(false);
    expect(renderer.dispatchNativeEvent(el.id + 100, 'press', event)).toBe(false);
    expect(received).toEqual([event]);
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
    expect(noFlush.pendingCount()).toBe(2); // CreateNode + SetStyle
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

    const el = r1.createElement('na-view');
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
    const el = renderer.createElement('na-text');
    renderer.setValue(el, 'hi');
    factory.end();
    expect(flushes.length).toBe(1);
    const batch = JSON.parse(flushes[0]!);
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
