/**
 * hmr.ts — shared hot-module-replacement contract.
 *
 * A module opts into HMR by registering a factory plus optional lifecycle
 * hooks. `replace()` snapshots the live instance, disposes it, creates the new
 * instance in the same JS realm, and restores the snapshot. No page reload,
 * JS-context reset or native-view teardown is implied.
 *
 * Apps that do not register hooks are not covered by the state-preservation
 * claim; this contract is deliberately explicit rather than assuming Angular
 * internals can be hot-swapped.
 */

export interface HmrHooks<T> {
  /** Capture local state (for example a signal value) before replacement. */
  snapshot?(instance: T): unknown;
  /** Restore captured state into the freshly created instance. */
  restore?(instance: T, state: unknown): void;
  /** Release resources owned by the outgoing instance. */
  dispose?(instance: T): void;
}

export interface HmrRegistration<T> {
  factory: () => T;
  hooks: HmrHooks<T>;
  instance: T;
  snapshot: unknown;
  generation: number;
}

/** Versioned reload message published by the dev server's `/hmr` stream. */
export interface HmrUpdate {
  readonly type: 'hmr';
  readonly moduleId: string;
  readonly version: number;
  readonly bundles: Readonly<Record<'web' | 'native', string>>;
}

export function isHmrUpdate(value: unknown): value is HmrUpdate {
  if (!value || typeof value !== 'object') return false;
  const update = value as Partial<HmrUpdate>;
  return (
    update.type === 'hmr' &&
    typeof update.moduleId === 'string' &&
    typeof update.version === 'number' &&
    Number.isFinite(update.version) &&
    !!update.bundles &&
    typeof update.bundles.web === 'string' &&
    typeof update.bundles.native === 'string'
  );
}

/**
 * Tiny synchronous runtime used by both the web adapter and embedded QuickJS.
 * All state is plain data, so it works in a restricted JS engine too.
 */
export class HmrRuntime {
  private readonly modules = new Map<string, HmrRegistration<unknown>>();

  /** Register the first version of a module. Returns false if id exists. */
  register<T>(id: string, factory: () => T, hooks: HmrHooks<T> = {}): boolean {
    if (this.modules.has(id)) return false;
    const instance = factory();
    this.modules.set(id, {
      factory: factory as () => unknown,
      hooks: hooks as HmrHooks<unknown>,
      instance,
      snapshot: hooks.snapshot?.(instance),
      generation: 1,
    });
    return true;
  }

  /**
   * Replace a registered module in place. The previous instance is snapshotted
   * before disposal; the new one receives that snapshot. Returns false when
   * the id was not registered (callers can fall back to a full reload).
   */
  replace<T>(id: string, factory: () => T, hooks: HmrHooks<T> = {}): boolean {
    const current = this.modules.get(id) as HmrRegistration<T> | undefined;
    if (!current) return false;

    const state = current.hooks.snapshot?.(current.instance);
    current.hooks.dispose?.(current.instance);
    const instance = factory();
    current.hooks.restore?.(instance, state);

    current.factory = factory;
    current.hooks = hooks;
    current.instance = instance;
    current.snapshot = state;
    current.generation += 1;
    return true;
  }

  has(id: string): boolean {
    return this.modules.has(id);
  }

  instance<T>(id: string): T | undefined {
    return this.modules.get(id)?.instance as T | undefined;
  }

  state(id: string): unknown {
    return this.modules.get(id)?.snapshot;
  }

  generation(id: string): number {
    return this.modules.get(id)?.generation ?? 0;
  }

  ids(): string[] {
    return [...this.modules.keys()];
  }

  clear(): void {
    this.modules.clear();
  }
}

/**
 * Apply a dev-server update once the caller has loaded the new module.
 * `load` returns the new factory/hooks from whichever bundle variant the
 * platform uses. Stale versions are ignored.
 */
export function applyHmrUpdate<T>(
  runtime: HmrRuntime,
  update: HmrUpdate,
  load: (update: HmrUpdate) => HmrModulePayload<T> | Promise<HmrModulePayload<T>>,
  lastVersion: { value: number },
): Promise<boolean> {
  if (update.version <= lastVersion.value) return Promise.resolve(false);
  return Promise.resolve(load(update)).then((payload) => {
    // Re-check after the async load: two overlapping events can both pass the
    // pre-check, and an older bundle must never win just because it resolved
    // last. lastVersion advances only on a successful replacement.
    if (update.version <= lastVersion.value) return false;
    const replaced = runtime.replace(update.moduleId, payload.factory, payload.hooks ?? {});
    if (replaced) lastVersion.value = update.version;
    return replaced;
  });
}

/** Shape produced by a dev HMR bundle on web or QuickJS. */
export interface HmrModulePayload<T> {
  readonly factory: () => T;
  readonly hooks?: HmrHooks<T>;
}
