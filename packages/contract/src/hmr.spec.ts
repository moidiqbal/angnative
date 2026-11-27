import { describe, it, expect } from 'vitest';
import {
  HmrRuntime,
  applyHmrUpdate,
  isHmrUpdate,
  type HmrUpdate,
} from './hmr';

interface Counter {
  value: number;
  label: string;
}

function counterFactory(initial: number, label: string): () => Counter {
  return () => ({ value: initial, label });
}

const counterHooks = {
  snapshot: (instance: Counter) => ({ value: instance.value }),
  restore: (instance: Counter, state: unknown) => {
    instance.value = (state as { value: number }).value;
  },
  dispose: () => {
    /* no external resources in the fixture */
  },
};

describe('shared HMR contract', () => {
  it('preserves component state across a module replacement', () => {
    const runtime = new HmrRuntime();
    runtime.register('counter', counterFactory(0, 'v1'), counterHooks);
    runtime.instance<Counter>('counter')!.value = 7;

    const replaced = runtime.replace('counter', counterFactory(0, 'v2'), counterHooks);
    expect(replaced).toBe(true);
    expect(runtime.instance<Counter>('counter')).toEqual({ value: 7, label: 'v2' });
    expect(runtime.state('counter')).toEqual({ value: 7 });
    expect(runtime.generation('counter')).toBe(2);
  });

  it('returns false for unregistered modules without creating state', () => {
    const runtime = new HmrRuntime();
    expect(runtime.replace('missing', counterFactory(1, 'x'))).toBe(false);
    expect(runtime.has('missing')).toBe(false);
    expect(runtime.generation('missing')).toBe(0);
  });

  it('applies a versioned update once and ignores stale versions', async () => {
    const runtime = new HmrRuntime();
    runtime.register('counter', counterFactory(0, 'v1'), counterHooks);
    runtime.instance<Counter>('counter')!.value = 3;
    const last = { value: 0 };
    const update: HmrUpdate = {
      type: 'hmr',
      moduleId: 'counter',
      version: 5,
      bundles: { web: '/dev-bundle.js?v=5', native: '/dev-bundle.native.js?v=5' },
    };

    let loads = 0;
    const load = () => {
      loads++;
      return { factory: counterFactory(0, 'v5'), hooks: counterHooks };
    };
    expect(await applyHmrUpdate(runtime, update, load, last)).toBe(true);
    expect(await applyHmrUpdate(runtime, { ...update, version: 4 }, load, last)).toBe(false);
    expect(loads).toBe(1);
    expect(runtime.instance<Counter>('counter')).toEqual({ value: 3, label: 'v5' });
    expect(last.value).toBe(5);
  });

  it('validates reload messages strictly', () => {
    expect(
      isHmrUpdate({
        type: 'hmr',
        moduleId: 'counter',
        version: 1,
        bundles: { web: '/a', native: '/b' },
      }),
    ).toBe(true);
    expect(isHmrUpdate({ type: 'hmr', moduleId: 'counter', version: 1 })).toBe(false);
    expect(isHmrUpdate(null)).toBe(false);
  });
});
