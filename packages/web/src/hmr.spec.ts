import { describe, it, expect } from 'vitest';
import { Window } from 'happy-dom';
import { HmrRuntime, type HmrUpdate } from '@angnative/contract/hmr';
import { DomBackend } from './dom-backend';
import { startWebHmr } from './hmr';

interface Counter {
  value: number;
  label: string;
  increment(): void;
}

function factory(value: number, label: string): () => Counter {
  return () => ({
    value,
    label,
    increment() {
      this.value += 1;
    },
  });
}

const hooks = {
  snapshot: (instance: Counter) => ({ value: instance.value }),
  restore: (instance: Counter, state: unknown) => {
    instance.value = (state as { value: number }).value;
  },
};

function update(version: number): HmrUpdate {
  return {
    type: 'hmr',
    moduleId: 'counter',
    version,
    bundles: { web: `/dev-bundle.js?v=${version}`, native: `/dev-bundle.native.js?v=${version}` },
  };
}

describe('web HMR client', () => {
  it('applies a reload without losing counter state and re-renders the DOM', async () => {
    const win = new Window({ url: 'http://localhost/' });
    const backend = new DomBackend(win.document as unknown as Document);
    backend.adoptRoot(1, win.document.body as unknown as Element);
    backend.apply([
      { kind: 'Create', node_id: 2, view_type: 'Text' },
      { kind: 'Append', parent_id: 1, child_id: 2 },
      { kind: 'SetText', node_id: 2, text: '0' },
    ]);

    const runtime = new HmrRuntime();
    runtime.register('counter', factory(0, 'v1'), hooks);
    const instance = runtime.instance<Counter>('counter')!;
    instance.increment();
    instance.increment();
    instance.increment();
    backend.apply([{ kind: 'SetText', node_id: 1, text: String(instance.value) }]);
    expect(backend.getNode(1)?.textContent).toBe('3');

    let emit: ((value: HmrUpdate) => void) | undefined;
    let applied = 0;
    const stop = startWebHmr<Counter>({
      runtime,
      moduleId: 'counter',
      subscribe: (handler) => {
        emit = handler;
        return () => {
          emit = undefined;
        };
      },
      loadModule: () => ({ factory: factory(0, 'v2'), hooks }),
      onApplied: (current) => {
        applied += 1;
        backend.apply([{ kind: 'SetText', node_id: 1, text: String(current.value) }]);
      },
    });

    emit!(update(2));
    // applyHmrUpdate resolves through a Promise chain.
    await Promise.resolve();
    await Promise.resolve();

    expect(applied).toBe(1);
    expect(runtime.generation('counter')).toBe(2);
    expect(runtime.instance<Counter>('counter')).toEqual({ value: 3, label: 'v2', increment: expect.any(Function) });
    expect(backend.getNode(1)?.textContent).toBe('3');

    emit!(update(1)); // stale version must be ignored
    await Promise.resolve();
    expect(applied).toBe(1);

    stop();
    expect(emit).toBeUndefined();
  });
});
