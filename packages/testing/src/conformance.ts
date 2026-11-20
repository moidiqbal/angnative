/**
 * conformance.ts — fixture harness helpers and the canonical expectation
 * vocabulary shared by the TS suite (see FIXTURES.md).
 */

import { readFileSync } from 'fs';
import { join } from 'path';
import { fileURLToPath } from 'url';
import { FakeBackend } from './fake-backend';

const __dirname = fileURLToPath(new URL('.', import.meta.url)).replace(/%20/g, ' ');
const FIXTURES_DIR = join(__dirname, '..', 'fixtures');

/** Per-node assertions understood by both the TS and Rust conformance runners. */
export interface NodeExpect {
  children?: number[];
  parentId?: number | null;
  textContent?: string | null;
  styles?: Record<string, string>;
  properties?: Record<string, unknown>;
  /**
   * True when the node's native view slot must survive a detach/reattach
   * cycle. The TS harness checks the node still exists and was created only
   * once (no destroy/recreate). See FIXTURES.md.
   */
  reused?: boolean;
}

/** Capability-degradation assertion: the feature must be diagnosed, never silent. */
export interface CapabilityExpect {
  nodeId: number;
  key: string;
  value?: unknown;
  /** When true (default), `key` must be recorded on the node. */
  diagnosed?: boolean;
}

export interface FixtureExpect {
  mutationKinds?: string[];
  finalTree?: Record<string, { children?: number[]; textContent?: string | null } | null>;
  nodes?: Record<string, NodeExpect>;
  capability?: CapabilityExpect;
  unknownElementThrows?: boolean;
  unknownStyleThrows?: boolean;
  /** Legacy alias of `unknownStyleThrows`, still asserted by the TS harness. */
  unknownKeyThrows?: boolean;
  missingFieldThrows?: boolean;
  badProtocolVersionThrows?: boolean;
}

export interface Fixture {
  name: string;
  description: string;
  commands: any[];
  expect?: FixtureExpect;
}

/** All top-level `expect` keys the TS harness asserts. */
export const SUPPORTED_EXPECT_KEYS: readonly string[] = [
  'mutationKinds',
  'finalTree',
  'nodes',
  'capability',
  'unknownElementThrows',
  'unknownStyleThrows',
  'unknownKeyThrows',
  'missingFieldThrows',
  'badProtocolVersionThrows',
];

/** All per-node `expect.nodes[id]` keys the TS harness asserts. */
export const SUPPORTED_NODE_KEYS: readonly string[] = [
  'children',
  'parentId',
  'textContent',
  'styles',
  'properties',
  'reused',
];

export function loadManifest(): string[] {
  const raw = readFileSync(join(FIXTURES_DIR, 'manifest.json'), 'utf8');
  return JSON.parse(raw) as string[];
}

export function loadFixture(name: string): Fixture {
  const raw = readFileSync(join(FIXTURES_DIR, `${name}.json`), 'utf8');
  return JSON.parse(raw) as Fixture;
}

export function runFixture(name: string) {
  const fixture = loadFixture(name);
  const backend = new FakeBackend();
  backend.runBatch(fixture.commands);
  return { fixture, result: backend.snapshot() };
}
