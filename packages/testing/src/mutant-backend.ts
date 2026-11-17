/**
 * mutant-backend.ts — 7 mutation "teeth" backends for the conformance suite.
 *
 * Each mutant extends FakeBackend with exactly ONE injected defect, mirroring
 * one of the 7 "fix before M1" items from the M0 review (TaskList T5):
 *
 *   1. Re-parent doesn't detach from old parent       (fake-node DOM fidelity)
 *   2. RemoveChild doesn't check child membership     (non-member removeChild)
 *   3. SetTextContent is a no-op                      (text/comment payloads)
 *   4. SetStyle silently accepts unknown keys         (§5.3: unknown → throw)
 *   5. CreateNode silently accepts unknown node types (unknown element)
 *   6. InsertChild ignores index, always appends      (tree integrity)
 *   7. SetProperty(null) keeps the property           (null removes property)
 *
 * The conformance suite must catch every mutant (suite goes RED) with exact
 * expected wrong values. The normal FakeBackend must stay GREEN on the same
 * fixtures. This proves the suite has teeth — it detects defects.
 */

import { FakeBackend } from './fake-backend';
import type { Command } from '@angnative/contract/protocol';
import { parseStyleValue } from '@angnative/contract/style';
import { NativeElement } from '@angnative/core/native-element';

/** Node types recognized by both the TS layer and the Rust host. */
const KNOWN_NODE_TYPES = new Set([
  'View',
  'Text',
  'Button',
  'Image',
  'ScrollView',
  'TextInput',
  'Custom',
]);

/* =========================================================================
 * Mutant 1 — Re-parent doesn't detach from old parent
 * M0 fix: fake-node DOM fidelity (re-parent)
 * Defect: InsertChild leaves the child in the old parent's children list.
 * ========================================================================= */
export class MutantReparentNoDetach extends FakeBackend {
  protected override apply(cmd: Command): string {
    if (cmd.kind === 'InsertChild') {
      const parent = this.registry.get(cmd.parentId);
      const child = this.registry.get(cmd.childId);

      // DEFECT: skip detaching from old parent — child stays in both lists.
      child.parentId = parent.id;
      const idx = Math.min(cmd.index, parent.children.length);
      parent.children.splice(idx, 0, child.id);

      return 'InsertChild';
    }
    return super.apply(cmd);
  }
}

/* =========================================================================
 * Mutant 2 — RemoveChild doesn't check child membership
 * M0 fix: non-member removeChild
 * Defect: RemoveChild removes the child from `parent` even if the child
 * belongs to a different parent (or no parent).
 * ========================================================================= */
export class MutantRemoveChildNoCheck extends FakeBackend {
  protected override apply(cmd: Command): string {
    if (cmd.kind === 'RemoveChild') {
      const child = this.registry.get(cmd.childId);

      // DEFECT: don't verify child.parentId === parentId before removing.
      // Detach from the child's actual parent (wrong — should have checked).
      if (child.parentId !== null) {
        const actualParent = this.registry.get(child.parentId);
        actualParent.removeChildId(child.id);
      }
      child.parentId = null;

      return 'RemoveChild';
    }
    return super.apply(cmd);
  }
}

/* =========================================================================
 * Mutant 3 — SetTextContent is a no-op
 * M0 fix: createNativeDocument keeping text/comment payloads
 * Defect: SetTextContent records the mutation kind but doesn't update
 * the node's textContent field.
 * ========================================================================= */
export class MutantSetTextNoOp extends FakeBackend {
  protected override apply(cmd: Command): string {
    if (cmd.kind === 'SetTextContent') {
      // DEFECT: do nothing — textContent is never set.
      return 'SetTextContent';
    }
    return super.apply(cmd);
  }
}

/* =========================================================================
 * Mutant 4 — SetStyle silently accepts unknown keys
 * M0 fix: §5.3 — unknown property writes must throw, not no-op
 * Defect: SetStyle skips parseStyleValue validation, so unknown keys are
 * stored without error.
 * ========================================================================= */
export class MutantSetStyleSilentUnknown extends FakeBackend {
  protected override apply(cmd: Command): string {
    if (cmd.kind === 'SetStyle') {
      const el = this.registry.get(cmd.id);
      // DEFECT: skip parseStyleValue — unknown key accepted silently.
      el.setStyle(cmd.key, cmd.value);
      return 'SetStyle';
    }
    return super.apply(cmd);
  }
}

/* =========================================================================
 * Mutant 5 — CreateNode silently accepts unknown node types
 * M0 fix: unknown element must throw
 * Defect: CreateNode skips the KNOWN_NODE_TYPES check.
 * ========================================================================= */
export class MutantCreateNodeSilentUnknown extends FakeBackend {
  protected override apply(cmd: Command): string {
    if (cmd.kind === 'CreateNode') {
      // DEFECT: skip KNOWN_NODE_TYPES check — unknown type accepted.
      const el = new NativeElement(cmd.id, cmd.nodeType.toLowerCase(), cmd.nodeType as any);
      this.registry.registerWithId(el);
      return 'CreateNode';
    }
    return super.apply(cmd);
  }
}

/* =========================================================================
 * Mutant 6 — InsertChild ignores index, always appends
 * M0 fix: tree integrity (re-parent / insert position)
 * Defect: InsertChild always appends to the end of children, ignoring `index`.
 * ========================================================================= */
export class MutantInsertChildWrongIndex extends FakeBackend {
  protected override apply(cmd: Command): string {
    if (cmd.kind === 'InsertChild') {
      const parent = this.registry.get(cmd.parentId);
      const child = this.registry.get(cmd.childId);

      // Detach from previous parent if re-parenting.
      if (child.parentId !== null && child.parentId !== parent.id) {
        const oldParent = this.registry.get(child.parentId);
        oldParent.removeChildId(child.id);
      }

      child.parentId = parent.id;

      // DEFECT: ignore `index`, always append.
      parent.children.push(child.id);

      return 'InsertChild';
    }
    return super.apply(cmd);
  }
}

/* =========================================================================
 * Mutant 7 — SetProperty(null) keeps the property
 * M0 fix: null value removes the property
 * Defect: SetProperty with value null stores null instead of removing.
 * ========================================================================= */
export class MutantSetPropertyNullKeeps extends FakeBackend {
  protected override apply(cmd: Command): string {
    if (cmd.kind === 'SetProperty') {
      const el = this.registry.get(cmd.id);
      // DEFECT: store null instead of removing the property.
      el.setProperty(cmd.key, cmd.value);
      return 'SetProperty';
    }
    return super.apply(cmd);
  }
}

/** All mutants, in order — one defect each. */
export const ALL_MUTANTS: ReadonlyArray<{
  name: string;
  defect: string;
  backend: FakeBackend;
}> = [
  {
    name: 'reparent-no-detach',
    defect: 'InsertChild does not detach child from old parent',
    backend: new MutantReparentNoDetach(),
  },
  {
    name: 'remove-child-no-check',
    defect: 'RemoveChild does not verify child belongs to parent',
    backend: new MutantRemoveChildNoCheck(),
  },
  {
    name: 'set-text-no-op',
    defect: 'SetTextContent does not update textContent',
    backend: new MutantSetTextNoOp(),
  },
  {
    name: 'set-style-silent-unknown',
    defect: 'SetStyle accepts unknown style keys without throwing',
    backend: new MutantSetStyleSilentUnknown(),
  },
  {
    name: 'create-node-silent-unknown',
    defect: 'CreateNode accepts unknown node types without throwing',
    backend: new MutantCreateNodeSilentUnknown(),
  },
  {
    name: 'insert-child-wrong-index',
    defect: 'InsertChild ignores index and always appends',
    backend: new MutantInsertChildWrongIndex(),
  },
  {
    name: 'set-property-null-keeps',
    defect: 'SetProperty(null) stores null instead of removing',
    backend: new MutantSetPropertyNullKeeps(),
  },
];
