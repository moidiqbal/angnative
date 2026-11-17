/**
 * @angnative/platform/testing — dev-only testing utilities guard.
 *
 * Exports `IS_TESTING_AVAILABLE` and `assertTestingAvailable()` so that
 * `@angular/core/testing` (JIT compiler paths, TestBed, etc.) never reaches a
 * production device bundle. See Task T5, M0 review item 4.
 */

export {
  IS_TESTING_AVAILABLE,
  assertTestingAvailable,
} from './testing-guard';
export type { } from './testing-guard';
