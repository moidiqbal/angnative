/**
 * conformance.spec.ts — the shared conformance suite (Task T3 / keystone).
 *
 * Loads JSON fixtures from fixtures/*.manifest, runs each command sequence
 * through the TS fake backend, and asserts the fixture's expect block.
 *
 * The same fixtures must be run by the Rust host (angnative-host-core) and
 * produce identical results (Task R4). This suite is the detector that keeps
 * TS and Rust honest — swap the backend, re-run, both must stay green.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { readFileSync } from 'fs';
import { join } from 'path';
import { fileURLToPath } from 'url';
import { FakeBackend, type FakeBackendResult } from './fake-backend';
import { encodeBatch, decodeBatch } from '@angnative/contract/protocol';

const __dirname = fileURLToPath(new URL('.', import.meta.url)).replace(/%20/g, ' ');
const FIXTURES_DIR = join(__dirname, '..', 'fixtures');

interface Fixture {
  name: string;
  description: string;
  commands: any[];
  expect?: {
    mutationKinds?: string[];
    finalTree?: Record<string, { children?: number[]; textContent?: string | null } | null>;
    nodes?: Record<string, NodeExpect>;
    unknownElementThrows?: boolean;
    unknownStyleThrows?: boolean;
    missingFieldThrows?: boolean;
    badProtocolVersionThrows?: boolean;
  };
}

interface NodeExpect {
  children?: number[];
  parentId?: number | null;
  textContent?: string | null;
  styles?: Record<string, string>;
  properties?: Record<string, unknown>;
}

/** Load the manifest (ordered list of fixture names). */
function loadManifest(): string[] {
  const raw = readFileSync(join(FIXTURES_DIR, 'manifest.json'), 'utf8');
  return JSON.parse(raw) as string[];
}

/** Load a single fixture by name. */
function loadFixture(name: string): Fixture {
  const raw = readFileSync(join(FIXTURES_DIR, `${name}.json`), 'utf8');
  return JSON.parse(raw) as Fixture;
}

function runFixture(name: string) {
  const fixture = loadFixture(name);
  const backend = new FakeBackend();
  backend.runBatch(fixture.commands);
  return { fixture, result: backend.snapshot() };
}

describe('conformance suite — shared fixtures (TS fake backend)', () => {
  const manifest = loadManifest();

  it(`loads ${manifest.length} fixtures`, () => {
    expect(manifest.length).toBeGreaterThan(0);
    for (const name of manifest) {
      expect(() => loadFixture(name)).not.toThrow();
    }
  });

  for (const name of manifest) {
    describe(name, () => {
      let fixture: Fixture;
      let result: FakeBackendResult;

      beforeEach(() => {
        ({ fixture, result } = runFixture(name));
      });

      it('runs without throwing', () => {
        expect(result).toBeDefined();
      });

      it('matches expected mutation kinds', () => {
        if (!fixture.expect?.mutationKinds) return;
        expect(result.batches.flat()).toEqual(fixture.expect.mutationKinds);
      });

      it('matches expected final tree shape', () => {
        if (!fixture.expect?.finalTree) return;
        const tree = fixture.expect.finalTree;
        for (const [idStr, expected] of Object.entries(tree)) {
          const id = Number(idStr);
          const node = result.nodes.get(id);
          if (expected === null) {
            expect(node).toBeUndefined();
            continue;
          }
          expect(node, `node ${id}`).toBeDefined();
          if (expected.children !== undefined) {
            expect(node!.children).toEqual(expected.children);
          }
          if (expected.textContent !== undefined) {
            expect(node!.textContent).toEqual(expected.textContent);
          }
        }
      });

      it('matches per-node expectations', () => {
        if (!fixture.expect?.nodes) return;
        const nodes = fixture.expect.nodes;
        for (const [idStr, expected] of Object.entries(nodes)) {
          const id = Number(idStr);
          const node = result.nodes.get(id);
          expect(node, `node ${id} exists`).toBeDefined();
          if (!node) continue;
          if (expected.children !== undefined) {
            expect(node.children).toEqual(expected.children);
          }
          if (expected.parentId !== undefined) {
            expect(node.parentId).toEqual(expected.parentId);
          }
          if (expected.textContent !== undefined) {
            expect(node.textContent).toEqual(expected.textContent);
          }
          if (expected.styles !== undefined) {
            for (const [key, value] of Object.entries(expected.styles)) {
              expect(node.styles.get(key)).toEqual(value);
            }
          }
          if (expected.properties !== undefined) {
            for (const [key, value] of Object.entries(expected.properties)) {
              expect(node.properties.get(key)).toEqual(value);
            }
          }
        }
      });

      it('throws on unknown element', () => {
        if (!fixture.expect?.unknownElementThrows) return;
        const backend = new FakeBackend();
        expect(() => {
          backend.runBatch([{ kind: 'CreateNode', id: 99, nodeType: 'Bogus' } as any]);
        }).toThrow();
      });

      it('throws on unknown style key', () => {
        if (!fixture.expect?.unknownStyleThrows) return;
        const backend = new FakeBackend();
        expect(() => {
          backend.runBatch([
            { kind: 'CreateNode', id: 1, nodeType: 'View' },
            { kind: 'SetStyle', id: 1, key: 'bogus-style', value: '1px' },
          ]);
        }).toThrow(/Unknown/);
      });

      it('throws on missing required field', () => {
        if (!fixture.expect?.missingFieldThrows) return;
        expect(() => {
          const backend = new FakeBackend();
          // @ts-expect-error deliberately malformed command
          backend.runBatch([{ kind: 'CreateNode', nodeType: 'View' }]);
        }).toThrow();
      });

      it('rejects bad protocol version at the wire layer', () => {
        if (!fixture.expect?.badProtocolVersionThrows) return;
        const bad = { protocolVersion: 99, frameId: 0, timestampMs: 0, commands: [] };
        expect(() => decodeBatch(encodeBatch(bad as any))).toThrow(
          /Unsupported protocol version/,
        );
      });
    });
  }
});
