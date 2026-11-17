/**
 * counter-signal.ts — shared signal state used by the demo.
 *
 * Signals work identically on native (no DOM) as they do in Angular web.
 * Change detection drives the native renderer through the command protocol.
 */

import { signal, computed, type WritableSignal } from '@angular/core';

export const counter: WritableSignal<number> = signal(0);

export const increment = () => counter.update((c) => c + 1);
export const decrement = () => counter.update((c) => c - 1);
export const reset = () => counter.set(0);

export const isEven = computed(() => counter() % 2 === 0);
export const isOdd = computed(() => counter() % 2 !== 0);
