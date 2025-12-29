# NoMercy Roadmap (v0.1 draft)

This roadmap distills the initial feature set from `spec.md` into actionable milestones for the first NoMercy iterations.

## Foundation: Engine & CLI
- Stand up `nomercy-core` crate with deterministic simulation primitives and schedule execution helper.
- Provide CLI scaffold (`nomercy-cli`) with commands: `beg`, `pray`, `replay`, `shrink`, `explore`.
- Enforce deterministic seed reporting and canonical CLI output layout (headings, `key=value` entries, `status=` footer).

## Determinism Qualification (`nomercy beg`)
- Static validation for nondeterministic APIs (time, randomness, environment access, unordered iteration).
- Manifest and checksum validation for adapters; refuse execution on drift.
- Exit code `4: system_not_deterministic` with actionable error messaging.

## Simulation Core (`nomercy pray`)
- Deterministic scheduler with logical steps and explicit fault ordering.
- Command lifecycle: `init` → `apply*` → crash/restore pairs → `observe` → `shutdown`.
- Protocol timeouts and replay handling per command with bounded retries.
- Canonical fault ordering and replay-safe shrink preference (fewer steps → fewer ops → fewer faults → earlier timing).

## Fault Injection
- Injected fault types: `crash@<step>`, `io_error@<step>`, `delay:<resource>@<step>+<duration>`.
- Deterministic normalization of schedules and replay-ready traces.
- Shrinking that preserves replayability and canonical ordering on ties.

## Observations & Invariants
- Deterministic observation payloads (canonical JSON, bounded size, stable shape).
- Canonical invariant representation with qualification on load; fatal on unknown/malformed predicates.
- Continuous invariant evaluation after apply/crash/restore with deterministic failure surfacing and repro capture.

## Reproduction & Shrinking
- Artifact layout under `./target/nomercy/<system>/` for traces and repros.
- `nomercy replay <repro.json>` enforces byte-identical reproduction; rejects `--seed`.
- `nomercy shrink <trace.json>` outputs minimized `repro.shrunk.json` and `trace.shrunk.json` with deterministic guidance.

## Adapters & Bindings
- Generated adapter bundles per system with `nomercy-adapter` entrypoint and `adapter.manifest.json`.
- Checksum validation and regeneration rules; refusal on drift or missing artifacts.
- Adapter purity: deterministic protocol translation only (no IO/time/env beyond manifest).

## Mock System: FlakySessions
- In-repo deterministic mock (`flaky_sessions`) implementing the full lifecycle (init/apply/crash/restore/observe).
- Used by CLI scaffolding and integration-style tests to validate engine flows without an external codebase.

## Language Bindings (future)
- Binding ergonomics for annotations/decorators while preserving canonical semantics.
- Automated adapter generation (Rust first), with manifest and checksum integration into the CLI flow.
