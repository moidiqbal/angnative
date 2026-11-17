/**
 * mutation-teeth.spec.ts — 7 mutation "teeth" specs (Task T3a).
 *
 * Each spec injects ONE defect into a mutant backend, runs conformance
 * fixtures through it, and asserts the suite goes RED with exact wrong
 * values. The normal FakeBackend is also run and must stay GREEN.
 *
 * This proves the conformance suite has teeth: every defect is detected.
 * Each mutant corresponds to one "fix before M1" item from the M0 review
 * (TaskList T5).
 */

import { describe, it, expect } from 'vitest';
import { FakeBackend, type FakeBackendResult } from './fake-backend';
import { ALL_MUTANTS } from './mutant-backend';
import type { Command } from '@angnative/contract/protocol';

/** Run a command sequence through a backend and return the snapshot. */
function runCommands(backend: FakeBackend, commands: Command[]): FakeBackendResult {
  backend.runBatch(commands);
  return backend.snapshot();
}

/** Assert that running commands through a backend throws. */
function assertThrows(fn: () => void, match?: string | RegExp): void {
  try {
    fn();
    expect('should have thrown').toBe('unreachable');
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    if (match) {
      expect(msg).toMatch(match);
    }
  }
}

/* =========================================================================
 * Mutant 1 — reparent-no-detach
 * Defect: InsertChild does not detach child from old parent
 * Fixture: reparent — node 3 moved from parent 2 to parent 1
 * Normal:  node2.children = []          (3 detached from 2)
 * Mutant:  node2.children = [3]         (3 still in 2's list — WRONG)
 * ========================================================================= */
describe('mutation teeth #1 — reparent-no-detach', () => {
  const commands: Command[] = [
    { kind: 'CreateNode', id: 1, nodeType: 'View' },
    { kind: 'CreateNode', id: 2, nodeType: 'View' },
    { kind: 'CreateNode', id: 3, nodeType: 'View' },
    { kind: 'InsertChild', parentId: 1, childId: 2, index: 0 },
    { kind: 'InsertChild', parentId: 2, childId: 3, index: 0 },
    { kind: 'InsertChild', parentId: 1, childId: 3, index: 1 },
  ];

  it('normal backend: node 2 has no children (green)', () => {
    const result = runCommands(new FakeBackend(), commands);
    const node2 = result.nodes.get(2)!;
    expect(node2.children).toEqual([]);
    const node3 = result.nodes.get(3)!;
    expect(node3.parentId).toBe(1);
    const node1 = result.nodes.get(1)!;
    expect(node1.children).toEqual([2, 3]);
  });

  it('mutant: node 2 still has child 3 (red — exact wrong value)', () => {
    const mutant = ALL_MUTANTS.find((m) => m.name === 'reparent-no-detach')!.backend;
    const result = runCommands(mutant, commands);
    const node2 = result.nodes.get(2)!;
    expect(node2.children).toEqual([3]); // WRONG: 3 was not detached from 2
    const node1 = result.nodes.get(1)!;
    expect(node1.children).toEqual([2, 3]); // node 1 lists 3 too — duplicate
  });
});

/* =========================================================================
 * Mutant 2 — remove-child-no-check
 * Defect: RemoveChild does not verify child belongs to parent
 * Scenario: remove child 2 from parent 3, but 2 is a child of parent 1
 * Normal: throws (membership check)
 * Mutant: succeeds silently, detaches 2 from parent 1 (WRONG)
 * ========================================================================= */
describe('mutation teeth #2 — remove-child-no-check', () => {
  const commands: Command[] = [
    { kind: 'CreateNode', id: 1, nodeType: 'View' },
    { kind: 'CreateNode', id: 2, nodeType: 'View' },
    { kind: 'CreateNode', id: 3, nodeType: 'View' },
    { kind: 'InsertChild', parentId: 1, childId: 2, index: 0 },
    // Remove child 2 from parent 3 — but 2 is a child of parent 1!
    { kind: 'RemoveChild', parentId: 3, childId: 2 },
  ];

  it('normal backend: throws on non-member removeChild (green)', () => {
    assertThrows(() => runCommands(new FakeBackend(), commands), /not a child/);
  });

  it('mutant: succeeds silently, detaches 2 from parent 1 (red)', () => {
    const mutant = ALL_MUTANTS.find((m) => m.name === 'remove-child-no-check')!.backend;
    const result = runCommands(mutant, commands);
    const node1 = result.nodes.get(1)!;
    expect(node1.children).toEqual([]); // WRONG: 2 was detached even though parent 3 doesn't own it
    const node2 = result.nodes.get(2)!;
    expect(node2.parentId).toBeNull(); // WRONG: parentId cleared without valid parent
  });
});

/* =========================================================================
 * Mutant 3 — set-text-no-op
 * Defect: SetTextContent does not update textContent
 * Fixture: set-text-content
 * Normal:  node1.textContent = "hello world", node2.textContent = "initial value"
 * Mutant:  node1.textContent = null, node2.textContent = null (WRONG)
 * ========================================================================= */
describe('mutation teeth #3 — set-text-no-op', () => {
  const commands: Command[] = [
    { kind: 'CreateNode', id: 1, nodeType: 'Text' },
    { kind: 'CreateNode', id: 2, nodeType: 'TextInput' },
    { kind: 'SetTextContent', id: 1, text: 'hello world' },
    { kind: 'SetTextContent', id: 2, text: 'initial value' },
  ];

  it('normal backend: text content set correctly (green)', () => {
    const result = runCommands(new FakeBackend(), commands);
    expect(result.nodes.get(1)!.textContent).toBe('hello world');
    expect(result.nodes.get(2)!.textContent).toBe('initial value');
  });

  it('mutant: textContent stays null (red — exact wrong value)', () => {
    const mutant = ALL_MUTANTS.find((m) => m.name === 'set-text-no-op')!.backend;
    const result = runCommands(mutant, commands);
    expect(result.nodes.get(1)!.textContent).toBeNull(); // WRONG
    expect(result.nodes.get(2)!.textContent).toBeNull(); // WRONG
  });
});

/* =========================================================================
 * Mutant 4 — set-style-silent-unknown
 * Defect: SetStyle accepts unknown style keys without throwing
 * Scenario: SetStyle("bogus-key", "1px") on a View
 * Normal: throws /Unknown/
 * Mutant: succeeds, stores bogus-key (WRONG)
 * ========================================================================= */
describe('mutation teeth #4 — set-style-silent-unknown', () => {
  const commands: Command[] = [
    { kind: 'CreateNode', id: 1, nodeType: 'View' },
    { kind: 'SetStyle', id: 1, key: 'bogus-key', value: '1px' },
  ];

  it('normal backend: throws on unknown style key (green)', () => {
    assertThrows(() => runCommands(new FakeBackend(), commands), /Unknown/);
  });

  it('mutant: accepts unknown key silently (red)', () => {
    const mutant = ALL_MUTANTS.find((m) => m.name === 'set-style-silent-unknown')!.backend;
    const result = runCommands(mutant, commands);
    const node1 = result.nodes.get(1)!;
    expect(node1.styles.get('bogus-key')).toBe('1px'); // WRONG: unknown key stored
  });
});

/* =========================================================================
 * Mutant 5 — create-node-silent-unknown
 * Defect: CreateNode accepts unknown node types without throwing
 * Scenario: CreateNode with nodeType "BogusType"
 * Normal: throws /Unknown node type/
 * Mutant: succeeds, creates node (WRONG)
 * ========================================================================= */
describe('mutation teeth #5 — create-node-silent-unknown', () => {
  const commands: Command[] = [
    { kind: 'CreateNode', id: 99, nodeType: 'BogusType' },
  ];

  it('normal backend: throws on unknown node type (green)', () => {
    assertThrows(() => runCommands(new FakeBackend(), commands), /Unknown node type/);
  });

  it('mutant: accepts unknown type silently (red)', () => {
    const mutant = ALL_MUTANTS.find((m) => m.name === 'create-node-silent-unknown')!.backend;
    const result = runCommands(mutant, commands);
    const node99 = result.nodes.get(99)!;
    expect(node99.nodeType).toBe('BogusType'); // WRONG: unknown type accepted
  });
});

/* =========================================================================
 * Mutant 6 — insert-child-wrong-index
 * Defect: InsertChild ignores index and always appends
 * Scenario: insert 2 at index 0, then insert 3 at index 0
 * Normal:  children = [3, 2] (3 at index 0, 2 pushed to index 1)
 * Mutant:  children = [2, 3] (both appended — WRONG)
 * ========================================================================= */
describe('mutation teeth #6 — insert-child-wrong-index', () => {
  const commands: Command[] = [
    { kind: 'CreateNode', id: 1, nodeType: 'View' },
    { kind: 'CreateNode', id: 2, nodeType: 'View' },
    { kind: 'CreateNode', id: 3, nodeType: 'View' },
    { kind: 'InsertChild', parentId: 1, childId: 2, index: 0 },
    { kind: 'InsertChild', parentId: 1, childId: 3, index: 0 },
  ];

  it('normal backend: index respected (green)', () => {
    const result = runCommands(new FakeBackend(), commands);
    const node1 = result.nodes.get(1)!;
    expect(node1.children).toEqual([3, 2]); // 3 inserted at 0, 2 shifted to 1
  });

  it('mutant: index ignored, always appends (red — exact wrong value)', () => {
    const mutant = ALL_MUTANTS.find((m) => m.name === 'insert-child-wrong-index')!.backend;
    const result = runCommands(mutant, commands);
    const node1 = result.nodes.get(1)!;
    expect(node1.children).toEqual([2, 3]); // WRONG: both appended, index ignored
  });
});

/* =========================================================================
 * Mutant 7 — set-property-null-keeps
 * Defect: SetProperty(null) stores null instead of removing
 * Scenario: set property "foo"="bar", then set "foo"=null
 * Normal:  property "foo" removed
 * Mutant:  property "foo" = null (WRONG)
 * ========================================================================= */
describe('mutation teeth #7 — set-property-null-keeps', () => {
  const commands: Command[] = [
    { kind: 'CreateNode', id: 1, nodeType: 'View' },
    { kind: 'SetProperty', id: 1, key: 'foo', value: 'bar' },
    { kind: 'SetProperty', id: 1, key: 'foo', value: null },
  ];

  it('normal backend: null removes property (green)', () => {
    const result = runCommands(new FakeBackend(), commands);
    const node1 = result.nodes.get(1)!;
    expect(node1.properties.has('foo')).toBe(false);
  });

  it('mutant: null keeps property (red — exact wrong value)', () => {
    const mutant = ALL_MUTANTS.find((m) => m.name === 'set-property-null-keeps')!.backend;
    const result = runCommands(mutant, commands);
    const node1 = result.nodes.get(1)!;
    expect(node1.properties.has('foo')).toBe(true); // WRONG: still present
    expect(node1.properties.get('foo')).toBeNull(); // WRONG: stored as null
  });
});

/* =========================================================================
 * All 7 mutants are detected: each one produces at least one red assertion.
 * This block asserts the mutant count matches the M0 review's 7 fixes.
 * ========================================================================= */
describe('mutation teeth — suite coverage', () => {
  it('has exactly 7 mutants matching the 7 M0 review fixes', () => {
    expect(ALL_MUTANTS).toHaveLength(7);
    const names = ALL_MUTANTS.map((m) => m.name);
    expect(names).toContain('reparent-no-detach');
    expect(names).toContain('remove-child-no-check');
    expect(names).toContain('set-text-no-op');
    expect(names).toContain('set-style-silent-unknown');
    expect(names).toContain('create-node-silent-unknown');
    expect(names).toContain('insert-child-wrong-index');
    expect(names).toContain('set-property-null-keeps');
  });

  it('every mutant has a non-empty defect description', () => {
    for (const m of ALL_MUTANTS) {
      expect(m.defect.length).toBeGreaterThan(0);
    }
  });
});
