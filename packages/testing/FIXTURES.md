# Conformance fixtures

The JSON files in `fixtures/` are shared by the TS fake backend
(`src/conformance.spec.ts`) and the Rust host runner
(`crates/angnative-host-view/tests/conformance_spec.rs`). The same commands
must produce the same tree state on both sides.

## Canonical element prefix

Native element tags use the canonical **`na-`** prefix (identity decision D-9):

`na-view`, `na-text`, `na-button`, `na-text-input`, `na-scroll-view`,
`na-image`, `na-pressable`.

Legacy `an-` / `native-` / bare HTML aliases (`div`, `span`, …) are accepted by
the renderer for backwards compatibility but must not appear in fixtures.
`fixtures/out/*` is generated output from `pnpm compile:fixtures` and also
uses `na-`. Fixture commands use the Rust `NodeType` names (`View`, `Text`,
`TextInput`, `Custom`, …) rather than tags; `Custom` is the host for `na-list`
and flattened component hosts.

## Supported `expect` keys

Only these top-level keys are asserted. An "unsupported expect key" guard test
in `conformance.spec.ts` fails if a fixture introduces any other key.

| Key | Assertion |
| --- | --- |
| `mutationKinds` | Exact ordered list of mutation kinds (`result.batches.flat()`). |
| `finalTree` | Map of node id → `{ children, textContent }` or `null` (node must be absent). |
| `nodes` | Per-node expectations (see below). |
| `capability` | `{ nodeId, key, value?, diagnosed? }` — asserts the capability property is recorded on the node (`diagnosed: true`, default), i.e. degradation is diagnosed, never silently dropped. Replaces the legacy `degradationDiagnosed` / `silentFailure` pair. |
| `unknownElementThrows` | `CreateNode` with an unknown node type throws. |
| `unknownStyleThrows` | `SetStyle` with an unknown style key throws. |
| `unknownKeyThrows` | Legacy alias of `unknownStyleThrows`; asserted with its own test. |
| `missingFieldThrows` | `CreateNode` without `id` throws. |
| `badProtocolVersionThrows` | Wire layer rejects a bad protocol version. |

Supported per-node keys inside `nodes["<id>"]`:

| Key | Assertion |
| --- | --- |
| `children` | Exact ordered child id list. |
| `parentId` | Exact parent id (or `null`). |
| `textContent` | Exact text (or `null`). |
| `styles` | Exact canonicalized style values for the listed keys. |
| `properties` | Exact property values (deep equality) for the listed keys. |
| `reused` | `true` requires the node to still exist **and** to have been created only once — its native view slot survived a `RemoveChild` / re-`InsertChild` cycle. Tracked via `FakeBackendResult.createCounts`. |

If an expectation cannot be represented with these keys, add a comment to the
fixture `description` and a dedicated spec in `packages/testing/src/`; do not
add an unasserted key to `expect`.

## Adding a fixture

1. Create `fixtures/<name>.json` with `name`, `description`, `commands`, and an
   `expect` block using only the keys above. Every fixture needs at least one
   behavioural assertion that can fail — `expect(result).toBeDefined()` does
   not count.
2. Add `"<name>"` to `fixtures/manifest.json` (order is preserved).
3. Run `pnpm --filter @angnative/testing test` and `pnpm test` from the repo root.
4. Mirror any new expectation key in the Rust runner so TS and Rust stay in
   lockstep. The Rust runner currently ignores `reused` and `capability` in its
   generic comparator (it has dedicated hard-coded tests for those scenarios),
   and has no equivalent of `unknownStyleThrows` / `unknownKeyThrows` or
   `missingFieldThrows` — see the hand-off notes for T4–T5.
