import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { createNativeAnimationPlayer } from './animation';

type Callback = (timestamp: number) => void;

const globals = globalThis as unknown as Record<string, unknown>;
const SAVED_KEYS = ['na_nowMs', 'requestAnimationFrame', 'cancelAnimationFrame'] as const;

let saved: Record<string, unknown> = {};
let now = 0;
let queue: Array<Callback | null> = [];

function installClock(): void {
  globals['na_nowMs'] = () => now;
  globals['requestAnimationFrame'] = (callback: Callback) => {
    queue.push(callback);
    return queue.length;
  };
  globals['cancelAnimationFrame'] = (id: number) => {
    if (id > 0 && id <= queue.length) queue[id - 1] = null;
  };
}

/** Advance the fake clock and run exactly the frames queued at this point. */
function flushFrame(atMs: number): void {
  now = atMs;
  const pending = queue.splice(0);
  for (const callback of pending) callback?.(0);
}

function opacityValues(values: Array<[string, string]>): string[] {
  return values.filter(([key]) => key === 'opacity').map(([, value]) => value);
}

describe('createNativeAnimationPlayer — host-clock easing', () => {
  beforeEach(() => {
    saved = {};
    for (const key of SAVED_KEYS) saved[key] = globals[key];
    now = 0;
    queue = [];
    installClock();
  });

  afterEach(() => {
    for (const key of SAVED_KEYS) {
      if (saved[key] === undefined) delete globals[key];
      else globals[key] = saved[key];
    }
  });

  it('applies ease-in opacity frames from the host clock', () => {
    const styles: Array<[string, string]> = [];
    let finished = 0;
    const player = createNativeAnimationPlayer(
      1,
      [{ offset: 0, opacity: 0 }, { offset: 1, opacity: 1 }],
      { duration: 100, easing: 'ease-in' },
      (key, value) => styles.push([key, value]),
    );
    player.addEventListener('finish', () => { finished++; });
    player.play();

    expect(opacityValues(styles)).toEqual(['0']);
    flushFrame(25);
    expect(opacityValues(styles)).toEqual(['0', '0.0625']);
    flushFrame(50);
    expect(opacityValues(styles)).toEqual(['0', '0.0625', '0.25']);
    flushFrame(75);
    expect(opacityValues(styles)).toEqual(['0', '0.0625', '0.25', '0.5625']);
    flushFrame(100);
    expect(opacityValues(styles)).toEqual(['0', '0.0625', '0.25', '0.5625', '1']);
    expect(player.playState).toBe('finished');
    expect(player.currentTime).toBe(100);
    expect(finished).toBe(1);
  });

  it('applies ease-out and ease-in-out curves', () => {
    const easeOut: string[] = [];
    const out = createNativeAnimationPlayer(
      2,
      [{ offset: 0, opacity: 0 }, { offset: 1, opacity: 1 }],
      { duration: 100, easing: 'ease-out' },
      (key, value) => { if (key === 'opacity') easeOut.push(value); },
    );
    out.play();
    flushFrame(50);
    expect(easeOut).toEqual(['0', '0.75']);

    // Start a fresh virtual timeline for the second player.
    now = 0;
    queue = [];
    const easeInOut: string[] = [];
    const inOut = createNativeAnimationPlayer(
      3,
      [{ offset: 0, opacity: 0 }, { offset: 1, opacity: 1 }],
      { duration: 100, easing: 'ease-in-out' },
      (key, value) => { if (key === 'opacity') easeInOut.push(value); },
    );
    inOut.play();
    flushFrame(25);
    flushFrame(75);
    expect(easeInOut).toEqual(['0', '0.125', '0.875']);
  });

  it('pause freezes elapsed time and cancel invalidates queued frames', () => {
    const styles: Array<[string, string]> = [];
    const player = createNativeAnimationPlayer(
      4,
      [{ offset: 0, opacity: 0 }, { offset: 1, opacity: 1 }],
      { duration: 100, easing: 'ease-in' },
      (key, value) => styles.push([key, value]),
    );
    player.play();
    flushFrame(25);
    player.pause();
    flushFrame(90);
    expect(opacityValues(styles)).toEqual(['0', '0.0625']);

    player.play();
    flushFrame(115);
    expect(opacityValues(styles)).toEqual(['0', '0.0625', '0.25']);

    player.cancel();
    flushFrame(500);
    expect(player.playState).toBe('idle');
    expect(opacityValues(styles)).toEqual(['0', '0.0625', '0.25']);
  });

  it('falls back to deterministic samples when the host clock is absent', async () => {
    delete globals['na_nowMs'];
    const styles: Array<[string, string]> = [];
    const player = createNativeAnimationPlayer(
      5,
      [{ offset: 0, opacity: 0 }, { offset: 1, opacity: 1 }],
      { duration: 100, easing: 'ease-in' },
      (key, value) => styles.push([key, value]),
    );
    player.play();
    expect(opacityValues(styles)).toEqual(['0', '0.5', '1']);
    await new Promise<void>((resolve) => queueMicrotask(resolve));
    expect(player.playState).toBe('finished');
  });

  it('fires finish only once when finish() follows a timed completion', () => {
    let finished = 0;
    const player = createNativeAnimationPlayer(
      6,
      [{ offset: 0, opacity: 0 }, { offset: 1, opacity: 1 }],
      { duration: 100, easing: 'ease-out' },
      () => {},
    );
    player.addEventListener('finish', () => { finished++; });
    player.play();
    flushFrame(100);
    player.finish();
    expect(finished).toBe(1);
  });
});
