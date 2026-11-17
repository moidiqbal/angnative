/**
 * testing-guard.ts — `@angular/core/testing` import guard (Task T5, M0 review item).
 *
 * Angular's testing package (`@angular/core/testing`) pulls in JIT compiler
 * paths and test-only utilities that must never reach a production device
 * bundle (spec: AOT only, `@angular/compiler` never reaches a runtime).
 *
 * This module exports an `IS_TESTING_AVAILABLE` boolean and an
 * `assertTestingAvailable()` helper. In production builds (`ngDevMode: false`),
 * Angular's own tree-shaking drops dev-only code paths, so any module that
 * imports `@angular/core/testing` through this guard contributes 0 bytes.
 *
 * Usage:
 *   import { assertTestingAvailable } from '@angnative/platform/testing';
 *   assertTestingAvailable('TestBed'); // throws in production
 */

import { isDevMode } from '@angular/core';

/** True only in dev mode. Production builds tree-shake dev-only paths away. */
export const IS_TESTING_AVAILABLE: boolean = isDevMode();

/**
 * Throw if a testing-only API is used in production.
 *
 * @param feature - Name of the testing feature being used (for the error message).
 */
export function assertTestingAvailable(feature: string): void {
  if (!IS_TESTING_AVAILABLE) {
    throw new Error(
      `${feature} is only available in development mode. ` +
        '`@angular/core/testing` is excluded from production builds (AOT only).',
    );
  }
}
