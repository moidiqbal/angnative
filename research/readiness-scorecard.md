# NativeAngular readiness scorecard

**Date:** 2026-11-12 · **Milestone:** M0 complete (spec §12) · **Weighted score: 14 / 100**

This is the instrument for the project's strategic factors. A factor's score moves
**only** when there is reproducible evidence for it — a passing contract test, a
published benchmark, a shipped release, an external adoption signal. Opinion does
not move a score, and neither does a plan.

The score is deliberately unflattering today. The M0 milestone proved the single
riskiest assumption in the thesis — that Angular can render to native views with
no DOM — and it proved it in Node, with no device, no Fabric and no ecosystem.
Almost everything that makes the project *win* is still ahead.

## The scorecard

| Factor | Weight | Score | Weighted | What the score is based on today | What raises it |
|---|---:|---:|---:|---|---|
| Angular-first developer experience | 20% | 1/5 | 4 | The platform runs real AOT Angular components from Angular's own API (`bootstrapNativeAngular`), and components are testable in Node in 251 ms. There is no CLI, no `create`, no hot reload, no device run | A `create` → run → ship path that works on a real device |
| Native module compatibility | 20% | 0/5 | 0 | Nothing exists. §5.7 states the intent; no spike, no adapter, no matrix | The M1.5 spike's measured answer + a published top-20 compatibility matrix |
| Production reliability | 20% | 1/5 | 4 | The renderer contract F4–F10 is implemented and asserted; the strict property rule turns silent failures into loud ones; CI is green and reproducible. Zero device hours, zero crash data, zero uptime | Contract suite green against the Fabric backend on device; crash-free telemetry from real apps |
| Performance | 15% | 0/5 | 0 | Only the JS payload is measured (90 KB gzip production baseline). **No device measurement of any kind** — startup, list scrolling, input latency and memory are all unmeasured | The §11.2 gate table, filled in against an RN baseline on the same device |
| Code reuse | 10% | 1/5 | 2 | The SPI is platform-neutral by construction and the same Angular app code targets every backend. The web ↔ native sharing story (shared business logic, validation, state, API clients) is asserted in the strategy but not demonstrated | A sample where the same services/validation run on web and on device |
| Tooling and documentation | 10% | 2/5 | 4 | CI with three jobs, pinned toolchain with a version check, governance files, design spec, implementation plan, gap analysis, constraint guard. No CLI, no docs site, no Angular Language Service support | `nativeangular doctor` / `dev` / `build`, starter templates, a docs site |
| Community adoption | 5% | 0/5 | 0 | Zero external contributors, zero plugins, zero adopters | The first external Angular team shipping a production app |

## How to read this

- **A 0 is not a criticism of the work; it is a statement about evidence.** M0's
  score is 14 because the milestone was a spike, and a spike's job is to remove
  architectural risk, not to score points on adoption.
- **The weights encode the strategy.** The three heaviest factors — DX, native
  module compatibility, production reliability — are all about an Angular team
  shipping a real app, which is why the audience priority is Angular enterprise
  teams first (§3).
- **The score is not a marketing number.** It is a planning instrument: the two
  factors sitting at 0/5 with 20% weight each are the two that decide whether
  NativeAngular is used by anyone outside this repository.
- **Every score must name its evidence.** A row without a reproducible artefact
  behind it is an opinion, and opinions do not earn points.

## What the next milestone must move

| Factor | Current | Target after M1 | The artefact that proves it |
|---|---:|---:|---|
| Angular-first DX | 1/5 | 3/5 | `npx create-nativeangular app` → runs on a device, with hot reload under the §11.1 budgets |
| Native module compatibility | 0/5 | 1/5 | M1.5 spike report + top-20 compatibility matrix (measured, published) |
| Production reliability | 1/5 | 2/5 | Contract suite green against Fabric on iOS + Android; device crash-free data |
| Performance | 0/5 | 1/5 | The §11.2 gate table filled in against an RN baseline |
| Tooling and docs | 2/5 | 3/5 | CLI shipped, starter template, docs site skeleton |
| Code reuse | 1/5 | 2/5 | One sample app sharing services between web and native |
| Community adoption | 0/5 | 0/5 | Nothing to claim yet — and claiming otherwise would be the first dishonest thing this project does |

## Link to the strategy

The factor list and weights come from the project's strategic assessment
(§3 goals, §12 allocation, §16 deferred decisions). Where the assessment and this
scorecard disagree, the scorecard wins — it is the one that carries evidence.
