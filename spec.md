# Specs

PROJECT: nomercy
LANGUAGE: Rust (engine + reference implementation)

PURPOSE:
  nomercy is a deterministic adversarial simulation engine written in Rust.
  It continuously searches for counterexamples in stateful systems by executing
  them under injected failures and invariant checking.
  Inspired by TigerBeetle-style simulation, not testing.

CORE PHILOSOPHY:
  - Simulation, not testing
  - Determinism over randomness
  - Invariants over assertions
  - Failures are explicit, injected, and replayable
  - One canonical engine, many language frontends
  - Designed for continuous (24/7) execution
  - Absence of failure means “not yet found”
  - If it isn’t injectable, it’s a bug

HIGH-LEVEL ARCHITECTURE:
  - nomercy-core (Rust):
      * Standalone CLI + engine
      * Scheduler, fault engine, invariant checker, shrinker
      * Single source of truth for semantics
  - Language bindings:
      * Provide ergonomics only (annotations, decorators, helpers)
      * Automatically generate adapters
      * Must not add or change semantics
  - Systems under simulation:
      * User-written application code
      * Never manually adapted by the user

KEY CLARIFICATION (IMPORTANT):
  - Users DO NOT write adapters.
  - Adapters are an internal implementation detail.
  - Adapters are generated automatically by language bindings.
  - If a user is aware of an adapter, the UX has failed.

SYSTEM MODEL:
  A system is a deterministic state machine with explicit crash boundaries.

  Logical capabilities (conceptual, not user-facing):
    - init(config) -> state
    - apply(op) -> state
    - crash()
    - restore(persistent_state) -> state
    - observe() -> observation

  Notes:
    - These capabilities are exposed via generated adapters
    - Users express them via language-native annotations or decorators
    - Scheduling, faults, and retries are controlled exclusively by nomercy
    - Anything not explicitly persisted is lost on crash

ADAPTER MODEL (AUTO-GENERATED):
  Definition:
    - An adapter is a generated executable or script that:
        * Wraps the user’s system
        * Speaks the nomercy protocol
        * Delegates all logic to user code

  Properties:
    - Generated automatically (build-time or run-time)
    - Inspectable artifacts
    - Protocol-compliant
    - Contain zero simulation logic
    - Contain zero fault logic
    - Deterministic pass-through only

  Responsibility split:
    - Adapter: protocol translation only
    - nomercy-core: authority over time, faults, crashes, invariants
    - User system: pure business logic + state transitions

PROTOCOL:
  - Transport: stdin / stdout
  - Encoding: line-delimited JSON
  - nomercy is authoritative; adapter/system is reactive

  Commands sent by nomercy:
    { "cmd": "init", "config": {...} }
    { "cmd": "apply", "op": {...} }
    { "cmd": "crash" }
    { "cmd": "restore", "state": {...} }
    { "cmd": "observe" }
    { "cmd": "shutdown" }

  Responses from adapter/system:
    { "ok": true }
    { "error": "...", "fatal": true }
    { "observation": {...} }

OBSERVATIONS:
  - Free-form JSON object
  - Deterministic and side-effect free
  - Used exclusively for invariant evaluation
  - Shape should be stable but is not enforced initially

INVARIANTS:
  - Defined outside the system
  - Evaluated by nomercy-core
  - Checked after:
      * every apply
      * every crash
      * every restore
  - Any violation immediately stops the run

  Invariant DSL (initial, declarative):
    - forall <path> <predicate>
    - sum(<path>) == <value>
    - equality and ordering only
    - no user-defined functions
    - deterministic evaluation only

FAULT MODEL:
  - All faults are deterministic and scheduled
  - No probabilistic or random failures

  Initial fault types:
    - crash@<step>
    - io_error@<step>
    - delay:<resource>@<step>+<duration>

  Faults:
    - Are injected only by nomercy
    - Are visible in traces
    - Must be shrinkable

SCHEDULER:
  - Step-based and deterministic
  - Single logical clock
  - No threads, sleeps, or wall-clock time
  - Same seed + config => identical execution

SIMULATION LOOP:
  - Choose seed
  - Generate or load fault schedule
  - Execute steps deterministically via adapter
  - Evaluate invariants continuously
  - On failure:
      * record full trace
      * shrink trace and fault schedule
      * emit minimal reproduction artifact

SHRINKING:
  - Fully automatic
  - Shrink axes (in order):
      1. fewer steps
      2. fewer operations
      3. fewer faults
      4. earlier fault timing
  - Output must always be exactly replayable

REPRODUCTION:
  - Failures emit a repro artifact (JSON)
  - Repro contains:
      * seed
      * fault schedule
      * minimal trace
      * invariant name
  - `nomercy replay <repro.json>` must reproduce exactly

CLI (PRIMARY INTERFACE):
  nomercy run <system>
  nomercy replay <repro.json>
  nomercy shrink <trace.json>
  nomercy explore <system>

  Common flags:
    --seed <n>
    --fault <fault>
    --invariants <file>
    --budget <steps|time|infinite>
    --ci
    --trace

  CLI guarantees:
    - Deterministic output
    - Copy-pasteable reproduction info
    - No interactive prompts
    - Minimal, focused failure output

LANGUAGE BINDINGS:
  Responsibilities:
    - Mark systems, operations, and observations
    - Generate adapters automatically
    - Serialize operations and observations
    - Invoke nomercy-core

  Non-responsibilities:
    - Scheduling
    - Fault injection
    - Shrinking
    - Invariant evaluation
    - Semantic decisions

  Rule:
    - Bindings may add ergonomics, never semantics

CONTINUOUS SIMULATION MODEL:
  - nomercy supports long-running (24/7) exploration
  - Intended to run outside CI as a background process
  - CI replays known repro corpus only
  - Bugs are treated as discovered counterexamples

NON-GOALS:
  - Probabilistic testing or fuzzing
  - Thread realism
  - Unit-test replacement
  - Implicit retries or magic behavior
  - User-written adapters

SLOGAN:
  “Deterministic universes. No mercy.”
