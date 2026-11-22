/**
 * animation.ts — deterministic Web Animations-style player for native nodes.
 *
 * Angular's `WebAnimationsDriver` calls `element.animate(keyframes, options)`.
 * AngNative elements delegate that call to a flat global registry keyed by a
 * numeric host id and node id. Nothing stored on the element closes over a
 * renderer or over the element itself, which keeps QuickJS object graphs free
 * of cycles (rquickjs GC cannot safely traverse them).
 *
 * The player applies three deterministic samples (start, midpoint, end). That
 * is intentional: the host has no wall-clock painter, so intermediate style
 * mutations are observable and reproducible instead of timing-dependent.
 */

export interface AnimationHost {
  readonly animationHostId: number;
  animateElement(nodeId: number, keyframes: unknown, options?: unknown): unknown;
}

export interface NativeAnimationPlayer {
  playState: string;
  currentTime: number;
  readonly finished: Promise<void>;
  play(): NativeAnimationPlayer;
  pause(): NativeAnimationPlayer;
  finish(): void;
  cancel(): void;
  addEventListener(type: string, listener: () => void): void;
  removeEventListener(type: string, listener: () => void): void;
}

interface Keyframe {
  offset: number;
  styles: Record<string, string | number>;
}

interface Sample {
  offset: number;
  styles: Record<string, string>;
}

const GLOBAL_HOSTS = '__angnativeAnimationHosts';
const GLOBAL_ANIMATE = '__angnativeAnimateElement';
const SAMPLE_OFFSETS: readonly number[] = [0, 0.5, 1];

function globalObject(): Record<string, unknown> {
  return globalThis as unknown as Record<string, unknown>;
}

function hostMap(): Map<number, AnimationHost> {
  const globals = globalObject();
  const existing = globals[GLOBAL_HOSTS];
  if (existing instanceof Map) {
    return existing as Map<number, AnimationHost>;
  }
  const created = new Map<number, AnimationHost>();
  globals[GLOBAL_HOSTS] = created;
  return created;
}

/** Register a renderer as the animation target for its native element ids. */
export function registerAnimationHost(host: AnimationHost): void {
  hostMap().set(host.animationHostId, host);
  const globals = globalObject();
  if (typeof globals[GLOBAL_ANIMATE] !== 'function') {
    globals[GLOBAL_ANIMATE] = function (
      hostId: number,
      nodeId: number,
      keyframes: unknown,
      options?: unknown,
    ): unknown {
      const target = hostMap().get(hostId);
      return target ? target.animateElement(nodeId, keyframes, options) : null;
    };
  }
}

/** Remove a renderer from the flat animation registry. */
export function unregisterAnimationHost(host: AnimationHost): void {
  hostMap().delete(host.animationHostId);
}

function normalizeKeyframes(input: unknown): Keyframe[] {
  if (!Array.isArray(input)) return [];
  const frames = input.map((raw, index) => {
    const source = raw && typeof raw === 'object'
      ? raw as Record<string, unknown>
      : {};
    const rawOffset = source['offset'];
    const offset = typeof rawOffset === 'number' && Number.isFinite(rawOffset)
      ? rawOffset
      : input.length === 1 ? 1 : index / (input.length - 1);
    const styles: Record<string, string | number> = {};
    for (const [key, value] of Object.entries(source)) {
      if (key === 'offset' || key === 'easing') continue;
      if (typeof value === 'string' || typeof value === 'number') {
        styles[key] = value;
      }
    }
    return { offset, styles };
  });
  frames.sort((a, b) => a.offset - b.offset);
  return frames;
}

function numericPart(value: string | number): { amount: number; unit: string } | null {
  const match = /^(-?(?:\d+\.?\d*|\.\d+))([a-z%]*)$/i.exec(String(value).trim());
  if (!match) return null;
  const amount = Number(match[1]);
  if (!Number.isFinite(amount)) return null;
  return { amount, unit: match[2] ?? '' };
}

function interpolate(from: string | number, to: string | number, t: number): string {
  const start = numericPart(from);
  const end = numericPart(to);
  if (start && end && start.unit === end.unit) {
    const amount = start.amount + (end.amount - start.amount) * t;
    return `${Number(amount.toFixed(4))}${start.unit}`;
  }
  return t < 0.5 ? String(from) : String(to);
}

function valueAt(property: string, offset: number, frames: Keyframe[]): string | undefined {
  const relevant = frames.filter((frame) => Object.hasOwn(frame.styles, property));
  if (relevant.length === 0) return undefined;
  let before = relevant[0]!;
  let after = relevant[relevant.length - 1]!;
  for (const frame of relevant) {
    if (frame.offset <= offset) before = frame;
    if (frame.offset >= offset) {
      after = frame;
      break;
    }
  }
  const start = before.styles[property]!;
  if (before === after || before.offset === after.offset) return String(start);
  const span = after.offset - before.offset;
  const t = Math.min(1, Math.max(0, (offset - before.offset) / span));
  return interpolate(start, after.styles[property]!, t);
}

function sampleFrames(frames: Keyframe[]): Sample[] {
  const properties = new Set<string>();
  for (const frame of frames) {
    for (const key of Object.keys(frame.styles)) properties.add(key);
  }
  return SAMPLE_OFFSETS.map((offset) => {
    const styles: Record<string, string> = {};
    for (const property of properties) {
      const value = valueAt(property, offset, frames);
      if (value !== undefined) styles[property] = value;
    }
    return { offset, styles };
  });
}

/** Create a player that drives native style mutations through `setStyle`. */
export function createNativeAnimationPlayer(
  nodeId: number,
  rawKeyframes: unknown,
  rawOptions: unknown,
  setStyle: (property: string, value: string) => void,
): NativeAnimationPlayer {
  const frames = normalizeKeyframes(rawKeyframes);
  const samples = sampleFrames(frames);
  const options = rawOptions && typeof rawOptions === 'object'
    ? rawOptions as Record<string, unknown>
    : {};
  const rawDuration = options['duration'];
  const duration = typeof rawDuration === 'number' && Number.isFinite(rawDuration)
    ? Math.max(0, rawDuration)
    : 0;
  const listeners: Array<() => void> = [];
  let playState = 'idle';
  let currentTime = 0;
  let finishFired = false;
  let resolveFinished: (() => void) | undefined;
  const finished = new Promise<void>((resolve) => {
    resolveFinished = resolve;
  });

  function applySample(sample: Sample): void {
    for (const [property, value] of Object.entries(sample.styles)) {
      setStyle(property, value);
    }
  }

  function emitFinish(): void {
    if (finishFired) return;
    finishFired = true;
    playState = 'finished';
    resolveFinished?.();
    for (const listener of listeners.slice()) listener();
  }

  function scheduleFinish(): void {
    const queue = globalObject()['queueMicrotask'] as ((callback: () => void) => void) | undefined;
    if (typeof queue === 'function') queue(emitFinish);
    else emitFinish();
  }

  function applyPosition(position: number): void {
    if (samples.length === 0) return;
    let closest = samples[0]!;
    for (const sample of samples) {
      if (Math.abs(sample.offset - position) < Math.abs(closest.offset - position)) {
        closest = sample;
      }
    }
    applySample(closest);
  }

  const player: NativeAnimationPlayer = {
    playState: 'idle',
    currentTime,
    finished,
    play(): NativeAnimationPlayer {
      finishFired = false;
      playState = 'running';
      for (const sample of samples) applySample(sample);
      currentTime = duration;
      playState = 'finished';
      scheduleFinish();
      return player;
    },
    pause(): NativeAnimationPlayer {
      playState = 'paused';
      return player;
    },
    finish(): void {
      const final = samples[samples.length - 1];
      if (final) applySample(final);
      currentTime = duration;
      emitFinish();
    },
    cancel(): void {
      playState = 'idle';
      currentTime = 0;
      finishFired = false;
    },
    addEventListener(type: string, listener: () => void): void {
      if (type === 'finish' && !listeners.includes(listener)) listeners.push(listener);
    },
    removeEventListener(type: string, listener: () => void): void {
      if (type !== 'finish') return;
      const index = listeners.indexOf(listener);
      if (index >= 0) listeners.splice(index, 1);
    },
  };

  Object.defineProperty(player, 'playState', {
    get: () => playState,
    enumerable: true,
    configurable: true,
  });

  Object.defineProperty(player, 'currentTime', {
    get: () => currentTime,
    set: (value: unknown) => {
      currentTime = typeof value === 'number' && Number.isFinite(value) ? value : 0;
      applyPosition(currentTime / Math.max(duration, 1));
    },
    enumerable: true,
    configurable: true,
  });

  return player;
}
