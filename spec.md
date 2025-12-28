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

  Artifact format (language bindings):
    - Bindings emit a self-contained adapter bundle with:
        * Executable entrypoint named `nomercy-adapter` (binary or script depending on language)
        * Adjacent manifest file `adapter.manifest.json`
        * Optional language-native wrapper files (e.g., generated Rust crate sources) stored under `src/` inside the bundle
    - Manifest contents:
        * Protocol + generator version
        * Supported operations and shapes (op names, argument schemas)
        * Config schema (JSON Schema) for `init(config)`
        * Hashes of generator inputs (source files, binding version, core version, protocol version)
        * Invocation metadata (expected working directory, environment knobs)
    - Invocation contract:
        * Entrypoint consumes manifest path via `--manifest <path>` and speaks over stdin/stdout
        * CLI runs: `./nomercy-adapter --manifest adapter.manifest.json`
        * Entrypoint must refuse to start if manifest hash check fails

  Example (Rust project using nomercy):
    - Binding produces a generated crate at `target/nomercy/adapters/<system_name>/` containing:
        * `src/adapter_main.rs` and glue code for annotations/derive macros
        * Cargo metadata wired to depend on `nomercy-core` protocol types only
        * Built binary at `target/nomercy/adapters/<system_name>/nomercy-adapter`
        * `adapter.manifest.json` describing ops/config schema derived from Rust attributes
    - CLI invocation during `nomercy run`:
        * If the binary is missing or the manifest hash mismatches, run `cargo run -p nomercy-adapter-<system_name> -- --manifest adapter.manifest.json`
        * Execution happens inside the workspace root so relative module paths resolve

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

  Generation timing and determinism:
    - Default flow: adapters are generated at build time (binding-specific build step) and validated on first CLI run
    - First-run safeguard: if no artifact exists or hashes drift, CLI triggers regeneration before executing schedules
    - Determinism enforcement:
        * Generator computes a checksum over: user-decorated sources, binding version, nomercy-core version, protocol version, generator flags, manifest schema
        * Checksum is stored in `adapter.manifest.json` and mirrored in a `adapter.checksum` file inside the bundle
        * CLI recomputes the checksum before each run; mismatch => refuse to execute stale adapter and regenerate
        * Build systems (e.g., Cargo) cache by checksum; identical inputs must produce byte-identical adapter binaries
    - Artifact location:
        * Repository-local: `./target/nomercy/adapters/<system_name>/` for build products committed to workspace cache (never checked in)
        * Workspace-local (non-repo runs): `~/.cache/nomercy/adapters/<system_name>/` as a fallback when build directories are ephemeral

  Failure surfacing rules (CLI UX):
    - Generation failure is fatal and must be reported before simulation begins
    - CLI error message includes:
        * Generator command executed and exit code
        * Path to captured stdout/stderr log (e.g., `target/nomercy/adapters/<system_name>/build.log`)
        * Next action the user can run verbatim (e.g., `cargo run -p nomercy-adapter-<system_name> -- --manifest adapter.manifest.json`)
        * Hash summary of inputs (binding/core/protocol versions) to confirm drift
    - CLI must not reuse stale artifacts on failure; it either regenerates successfully or aborts
    - Logs are kept alongside the adapter bundle and referenced directly in the failure message
    - Retry guidance must be copy-pasteable and deterministic (no “maybe” advice)

PROTOCOL:
  - Transport: stdin / stdout
  - Encoding: line-delimited JSON
  - nomercy is authoritative; adapter/system is reactive

  Protocol versioning:
    - All commands include a required `version` field (semantic version string)
    - Engine sends its current version with every request; adapters must respond with the same version
    - Mismatched or missing versions are fatal: engine aborts the session and records a repro

  Command lifecycle and shutdown semantics:
    - `shutdown` means: stop accepting new commands, flush any buffered output, exit cleanly
    - “End of run” means: engine finished the current schedule and will either issue `shutdown` or start a new replay/run; no additional side effects are implied
    - Engines never infer shutdown from EOF; explicit `shutdown` is required for clean teardown

  Commands sent by nomercy:
    { "version": "x.y.z", "cmd": "init", "config": {...} }
    { "version": "x.y.z", "cmd": "apply", "op": {...} }
    { "version": "x.y.z", "cmd": "crash" }
    { "version": "x.y.z", "cmd": "restore", "state": {...} }
    { "version": "x.y.z", "cmd": "observe" }
    { "version": "x.y.z", "cmd": "shutdown" }

  Error handling and response schema:
    - Success: { "ok": true }
    - Retryable error: { "error": "...", "retryable": true, "fatal": false }
    - Fatal error: { "error": "...", "fatal": true }
    - Observation response (unchanged): { "observation": {...} }
    - Unknown fields in adapter responses are ignored but recorded in trace for debugging
    - Invalid JSON or missing required fields => fatal; engine aborts and emits repro

    Example responses:
      - Retryable: { "error": "transient IO", "retryable": true, "fatal": false }
      - Fatal: { "error": "state divergence", "fatal": true }

    Engine decisions (simplified):
      | Condition                                 | Engine action                |
      |-------------------------------------------|-----------------------------|
      | Retryable error on apply/init/observe     | Retry command (bounded)     |
      | Fatal error flag                          | Abort run, emit repro       |
      | Invalid/malformed JSON                    | Abort run, emit repro       |
      | Version mismatch                          | Abort session, emit repro   |
      | Max retries exceeded                      | Abort run, emit repro       |

  Adapter timeouts & backpressure:
    - Max bytes per line: 64 KiB (lines exceeding are truncated and marked)
    - Max response latency: 5s default per command (configurable); exceeding triggers timeout
    - On timeout: engine treats as retryable once; repeated timeout becomes fatal
    - On truncation/partial write: engine marks response as incomplete, aborts current run, and records repro with raw line
    - Adapters must flush stdout after every response; engine never waits for stderr

  JSON validity and unknown fields:
    - Lines must be well-formed JSON objects
    - Unknown fields are tolerated but not acted upon; schema-required fields must be present
    - Fields with wrong types are treated as malformed JSON => fatal abort

  Idempotency and retries:
    - `apply` must be idempotent across retries: identical command replays must not produce diverging state
    - Engine may retry `apply` after retryable errors or timeouts; adapters must ensure apply replay is safe
    - `init`, `restore`, and `observe` are treated as pure/side-effect-free relative to retries

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
