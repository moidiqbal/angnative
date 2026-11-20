/**
 * conformance.spec.ts — the shared conformance suite (Task T3 / keystone).
 *
 * Loads JSON fixtures from fixtures/*.json, runs each command sequence
 * through the TS fake backend, and asserts the fixture's expect block.
 *
 * The same fixtures must be run by the Rust host (angnative-host-core) and
 * produce identical results (Task R4). This suite is the detector that keeps
 * TS and Rust honest — swap the backend, re-run, both must stay green.
 *
 * Only the keys listed in SUPPORTED_EXPECT_KEYS / SUPPORTED_NODE_KEYS are
 * asserted. A guard test fails loudly if a fixture introduces an unknown key
 * (see FIXTURES.md).
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { decodeBatch, encodeBatch } from '@angnative/contract/protocol';
import { FakeBackend, type FakeBackendResult } from './fake-backend';
import {
  loadManifest,
  loadFixture,
  runFixture,
  SUPPORTED_EXPECT_KEYS,
  SUPPORTED_NODE_KEYS,
  type Fixture,
} from './conformance';

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

      it('uses only supported expectation keys', () => {
        const expectBlock = fixture.expect ?? {};
        for (const key of Object.keys(expectBlock)) {
          expect(
            SUPPORTED_EXPECT_KEYS,
            `fixture ${name}: unsupported expect key "${key}"`,
          ).toContain(key);
        }
        for (const [idStr, node] of Object.entries(expectBlock.nodes ?? {})) {
          for (const key of Object.keys(node)) {
            expect(
              SUPPORTED_NODE_KEYS,
              `fixture ${name} node ${idStr}: unsupported node key "${key}"`,
            ).toContain(key);
          }
        }
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
          if (expected.reused !== undefined) {
            const createCount = result.createCounts.get(id) ?? 0;
            if (expected.reused) {
              // The slot survived the detach: node is alive and was never
              // destroyed/recreated (a recreate bumps the create count).
              expect(node, `node ${id} view slot retained`).toBeDefined();
              expect(
                createCount,
                `node ${id} created once (native view slot reused)`,
              ).toBe(1);
            } else {
              expect(
                createCount === 0 || createCount > 1,
                `node ${id} view slot was not reused`,
              ).toBe(true);
            }
          }
        }
      });

      it('records capability degradation (never silent)', () => {
        if (!fixture.expect?.capability) return;
        const { nodeId, key, value = true, diagnosed = true } = fixture.expect.capability;
        const node = result.nodes.get(nodeId);
        expect(node, `node ${nodeId}`).toBeDefined();
        if (diagnosed) {
          expect(
            node!.properties.get(key),
            `capability "${key}" diagnosed (not silently dropped)`,
          ).toEqual(value);
        } else {
          expect(node!.properties.has(key), `capability "${key}" not recorded`).toBe(false);
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

      it('throws on unknown key (legacy alias)', () => {
        if (!fixture.expect?.unknownKeyThrows) return;
        const backend = new FakeBackend();
        expect(() => {
          backend.runBatch([
            { kind: 'CreateNode', id: 1, nodeType: 'View' },
            { kind: 'SetStyle', id: 1, key: 'bogus-key', value: '1px' },
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
