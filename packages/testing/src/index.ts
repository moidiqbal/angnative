/**
 * @angnative/testing — shared conformance suite + fake backend.
 *
 * The keystone of the AngNative plan (TaskList §4): the same JSON fixtures
 * run on the TS fake backend and the Rust host, both must pass. This keeps
 * every new platform shell honest without rewriting tests.
 *
 * Exports:
 *   - FakeBackend — TS implementation of the TS↔Rust boundary
 *   - runFixture / loadFixture — fixture harness helpers
 *   - NodeSnapshot, FakeBackendResult — observation types
 */

export { FakeBackend, type FakeBackendResult, type NodeSnapshot } from './fake-backend';
export {
  loadManifest,
  loadFixture,
  runFixture,
  SUPPORTED_EXPECT_KEYS,
  SUPPORTED_NODE_KEYS,
  type Fixture,
  type FixtureExpect,
  type NodeExpect,
  type CapabilityExpect,
} from './conformance';
export {
  MutantReparentNoDetach,
  MutantRemoveChildNoCheck,
  MutantSetTextNoOp,
  MutantSetStyleSilentUnknown,
  MutantCreateNodeSilentUnknown,
  MutantInsertChildWrongIndex,
  MutantSetPropertyNullKeeps,
  ALL_MUTANTS,
} from './mutant-backend';
