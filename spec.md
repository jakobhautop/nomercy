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
    - Scheduling, faults, and command replays are controlled exclusively by nomercy
    - Anything not explicitly persisted is lost on crash

DETERMINISM CONTRACT (NORMATIVE):
  A system is deterministic under nomercy if and only if:
    1. Identical command sequences produce byte-identical crash persisted state and observations.
    2. Execution does not depend on wall-clock time, randomness, or ambient process state (environment variables, locale, filesystem layout).
    3. Iteration over unordered collections is forbidden.
    4. Floating-point behavior is deterministic across executions and platforms, or floating-point types are avoided entirely.
    5. All serialization is canonical and stable.
  Violations:
    - Detected by bindings before execution
    - Rejected prior to running any schedules
    - Never deferred to runtime simulation

DETERMINISM QUALIFICATION (MANDATORY PRE-RUN):
  Command:
    - `nomercy beg <system>`
  Purpose:
    - Statistically and structurally validate determinism requirements
    - Fail before any simulation, fault injection, or invariant evaluation
  Rules:
    - Systems that fail qualification MUST NOT be executed
    - Qualification failure is a system error, not a simulation failure
    - Qualification is binding-defined and engine-enforced
  Qualification checks (non-exhaustive, binding-specific):
    - Use of nondeterministic APIs (time, randomness, UUIDs, ambient environment)
    - Iteration over unordered collections
    - Non-canonical or unstable serialization (including observation serialization determinism)
    - Crash/restore payload determinism and closure
    - Presence of undeclared side effects
  CLI behavior:
    - `nomercy pray` implicitly performs qualification if it has not already succeeded
    - Qualification failures abort immediately with a system error
  Exit code:
    - 4: system_not_deterministic

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
    - CLI invocation during `nomercy pray`:
        * If the binary is missing or the manifest hash mismatches, run `cargo run -p nomercy-adapter-<system_name> -- --manifest adapter.manifest.json`
        * Execution happens inside the workspace root so relative module paths resolve

  Properties:
    - Generated automatically (build-time or run-time)
    - Inspectable artifacts
    - Protocol-compliant
    - Contain zero simulation logic
    - Contain zero fault logic
    - Deterministic pass-through only

  Adapter Purity Rules:
    - MUST NOT:
        * Generate identifiers
        * Read system or monotonic time
        * Access environment variables except those explicitly declared in the manifest
        * Perform filesystem, network, or other IO beyond stdin/stdout
        * Perform command replays, caching, buffering, or speculative execution
        * Contain simulation, fault, scheduling, or invariant logic
    - MUST:
        * Forward each engine command exactly once
        * Emit exactly one response per command
        * Flush stdout after every response
        * Behave purely as protocol translation
    - Violations are treated as protocol violations and abort execution immediately.

  Responsibility split:
    - Adapter: protocol translation only
    - nomercy-core: authority over time, faults, crashes, invariants
    - User system: pure business logic + state transitions

TERMINOLOGY NORMALIZATION:
  - “test failure” => “counterexample found”
  - “retry” => “command replay”
  - “timeout” => “protocol timeout”
  - “flaky” => MUST NOT appear
  - “test run” => “simulation run”
  - All CLI output and documentation MUST adopt this language consistently.

  Generation timing and determinism:
    - Default flow: adapters are generated at build time (binding-specific build step) and validated on first CLI run
    - First-run safeguard: if no artifact exists or hashes drift, CLI triggers regeneration before executing schedules
    - Determinism enforcement:
        * Generator computes a checksum over: user-decorated sources, binding version, nomercy-core version, protocol version, generator flags, manifest schema. The checksum is the SHA-256 digest of the concatenation of these inputs encoded in canonical UTF-8 JSON, with the manifest serialized **excluding** the `checksum` field to avoid self-reference.
        * Authoritative checksum location for validation is `adapter.checksum`; `adapter.manifest.json` includes the same value in a `checksum` field for debugging only. CLI compares its recomputed digest against `adapter.checksum` and refuses execution if they differ.
        * Regeneration and refusal rules:
            - If `adapter.checksum` is missing, mismatched, or cannot be parsed, CLI regenerates the adapter; if regeneration fails, it aborts with exit code 3.
            - If `adapter.manifest.json` embeds a checksum that disagrees with `adapter.checksum`, CLI treats the adapter as stale and regenerates from sources; regeneration rewrites both files so they converge.
            - Binding generators must compute the checksum with the manifest’s `checksum` field set to null/absent in the digest input to prevent recursion.
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
    - Command replay guidance must be copy-pasteable and deterministic (no “maybe” advice)

PROTOCOL:
  - Transport: stdin / stdout
  - Encoding: line-delimited JSON
  - nomercy is authoritative; adapter/system is reactive

  Protocol versioning:
    - All commands include a required `version` field (semantic version string).
    - Engine sends its current version with every request; adapters must respond with the same `version` value in every response object (including errors and observations).
    - Mismatched or missing versions are fatal: engine aborts the session and records a repro; validation rejects responses where `version` is absent, null, or differs from the request.

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
    - Success: { "version": "x.y.z", "ok": true }
    - Replayable error (command replay allowed): { "version": "x.y.z", "error": "...", "retryable": true, "fatal": false }
    - Fatal error: { "version": "x.y.z", "error": "...", "fatal": true }
    - Observation response: { "version": "x.y.z", "observation": {...} }
    - Crash response: { "version": "x.y.z", "ok": true, "state": {...} } where `state` is the persisted payload that restore consumes.
    - Restore acknowledgment: { "version": "x.y.z", "ok": true }
    - Unknown fields in adapter responses are ignored but recorded in trace for debugging
    - Invalid JSON or missing required fields (including `version`) => fatal; engine aborts and emits repro

    Example responses:
      - Replayable: { "version": "1.2.3", "error": "transient IO", "retryable": true, "fatal": false }
      - Fatal: { "version": "1.2.3", "error": "state divergence", "fatal": true }
      - Observation: { "version": "1.2.3", "observation": { "balances": { "alice": 10 } } }
      - Crash with persisted state: { "version": "1.2.3", "ok": true, "state": { "disk": { ... } } }

    Engine decisions (simplified):
      | Condition                                  | Engine action                 |
      |--------------------------------------------|------------------------------|
      | Replayable error on init/apply/observe     | Command replay (bounded)     |
      | Protocol timeout on replayable command     | Command replay once, then fatal      |
      | Fatal error flag                           | Abort run, emit repro        |
      | Invalid/malformed JSON                     | Abort run, emit repro        |
      | Version mismatch                           | Abort session, emit repro    |
      | Max command replays exceeded               | Abort run, emit repro        |

  Adapter protocol timeouts & backpressure:
    - Max bytes per line: 64 KiB (lines exceeding are truncated and marked)
    - Max response latency: 5s default per command (configurable); exceeding triggers a protocol timeout
    - On protocol timeout: engine permits a single command replay; repeated timeout becomes fatal. Timeout replay captures the raw command, missing response marker, and whether a replay was issued in the trace.
    - On truncation/partial write: engine marks response as incomplete, aborts current run, and records repro with raw line
    - Adapters must flush stdout after every response; engine never waits for stderr

  JSON validity and unknown fields:
    - Lines must be well-formed JSON objects
    - Unknown fields are tolerated but not acted upon; schema-required fields must be present
    - Fields with wrong types are treated as malformed JSON => fatal abort

  Idempotency and command replays:
    - `apply` must be idempotent across command replays: identical command replays must not produce diverging state.
    - Command replay + idempotency matrix:
        * `init`: may be replayed once on replayable error/protocol timeout; must recreate identical initial state and persisted artifacts.
        * `apply`: may be replayed; must be idempotent with respect to both volatile and persisted state.
        * `restore`: may be replayed; must be a pure function of the provided `state` and must not attempt external recovery side effects.
        * `observe`: may be replayed; must be read-only and deterministic.
        * `crash`: may be replayed; must emit identical `state` payloads across replays and must not attempt to continue execution after emitting state.
        * `shutdown`: must not be replayed; protocol timeouts are treated as fatal.
    - Engine may replay `init`, `apply`, `restore`, `observe`, and `crash` once after replayable errors or protocol timeouts; adapters must ensure replay is safe. Non-replayable/fatal errors abort immediately.

  Responses from adapter/system must echo `version`:
    { "version": "x.y.z", "ok": true }
    { "version": "x.y.z", "error": "...", "fatal": true }
    { "version": "x.y.z", "observation": {...} }
    { "version": "x.y.z", "ok": true, "state": {...} }

  Crash state capture and restore input shape:
    - Crash responses MUST include `state` when persistence exists; `state` is a JSON object representing the complete persisted view required to restart the system. Empty objects are allowed when no persistence is present.
    - Restore requests replay the exact payload previously emitted by crash: { "version": "x.y.z", "cmd": "restore", "state": <state-object> }.
    - Engine behavior:
        * Every crash response line is recorded verbatim in the trace, including `state`; repros persist the latest crash `state` used during the failing schedule and include it in both `trace.json` and `repro.json`.
        * On protocol timeout before a crash response, engine records a timeout marker and may command replay once; if a replay succeeds, the persisted `state` from the successful replay is what traces/repros capture.
        * Mismatched/malformed `state` during restore (type mismatch, missing required keys per manifest) is fatal and recorded with the offending payload.
    - Adapters must guarantee that repeated crash executions under command replay produce byte-identical `state` payloads so restore replay is deterministic.

OBSERVATIONS:
  - Free-form JSON object
  - Deterministic and side-effect free
  - Used exclusively for invariant evaluation
  - Stability rules:
      * Shape must be forward-compatible across an entire run: keys cannot disappear, and field types cannot change once a run begins.
      * Breaking changes (key removal/type change) require a versioned observation name (e.g., `balances.v2`) that coexists with prior versions for the duration of the run.
      * Observation producers must tolerate replay: serializing the same state twice yields byte-identical JSON.
      * Observations consumed by long-running simulations must remain backward compatible for the run duration; migrations happen between runs via explicit version bumps.
  - Deterministic serialization requirements:
      * Canonical JSON: stable key ordering, deterministic number formatting, and no incidental fields (timestamps, random IDs).
      * No binary blobs; payloads must be UTF-8 JSON and should avoid base64 unless essential.
      * Observations must not exceed limits: 256 KiB per observation, max nesting depth of 8, and no arrays longer than 10,000 elements. Violations are fatal protocol errors; engine aborts the run, records the offending observation (truncated to 64 KiB for logging only), and writes the failure into the trace/repro with a `reason=observation_limit` marker.
      * Serialization is pure: identical input state yields byte-identical output, including field ordering and whitespace.
  - Recommended payload limits:
      * Prefer summarized counts over unbounded lists; explicitly document truncation behavior if applied.
      * If truncation occurs, include deterministic markers (e.g., `"truncated": true`, `"omitted": 42`).
      * Avoid embedding unbounded histories; favor snapshots with deterministic ordering.

INVARIANTS:
  - Defined outside the system
  - Evaluated by nomercy-core
  - Checked after:
      * every apply
      * every crash
      * every restore
  - Any violation immediately stops the run
  - Naming rules:
      * Required `snake_case` identifier; optional namespace via dot segments (e.g., `ledger.balance_nonnegative`, `session.always_progress`).
      * Names are immutable within a run; changes require a new invariant entry.
  - Failure surfacing:
      * Repro artifacts must record the failing invariant name, predicate, observation snapshot, and human-readable failure message.
      * Failure messages are deterministic, single-line strings that reference concrete values (no flakiness hints like "maybe").
      * Repro stores both pre- and post-shrink invariant failures with the same structure for byte-identical replay.
      * Repro invariants section shape:
          - `invariants`: array of objects with `name`, `predicate`, `message`, and `observation` as captured at failure.
          - Each entry also records `step` and `fault_schedule` references for replay.

  Invariant representation (canonical, binding-friendly):
    - User-facing APIs are language-native (e.g., Rust macros, decorators); users never write a separate DSL.
    - Bindings compile language-native predicates into a canonical declarative form consumed by nomercy-core.
    - Canonical predicate grammar (JSON shape, not a text DSL):
        * All predicates are JSON objects tagged by `kind`.
        * Path syntax uses JSONPath-like segments with dot-delimited keys and `*` for map wildcards, `[*]` for arrays. Paths are deterministic and must not include filters or arithmetic.
        * Quantifiers and aggregations operate over paths that resolve to arrays or objects.
        * Allowed `kind` values and shapes (JSON Schema style):
            - Equality/ordering:
              `{ "kind": "cmp", "op": "eq|ne|lt|lte|gt|gte", "left": <expr>, "right": <expr> }`
            - Logical combinators:
              `{ "kind": "and", "predicates": [<predicate>, ...] }`
              `{ "kind": "or", "predicates": [<predicate>, ...] }`
              `{ "kind": "not", "predicate": <predicate> }`
            - Quantifier:
              `{ "kind": "forall", "path": "<json-path>", "predicate": <predicate> }`
            - Aggregation:
              `{ "kind": "aggregate", "agg": "sum|min|max|count", "path": "<json-path>", "op": "eq|ne|lt|lte|gt|gte", "value": <number> }`
            - Constant/field references:
              Expressions inside `cmp.left/right` can be literals (`null`, boolean, number, string) or `{ "kind": "field", "path": "<json-path>" }`.
    - Serialization rules:
        * Canonical predicates are serialized as canonical JSON with stable key ordering and no extraneous fields.
        * Paths are required to be absolute within the observation root; leading `$` is rejected to avoid duplicates (`balances.*` not `$..balances.*`).
        * Aggregations operate over numeric values only; non-numeric elements cause validation failure at load time.
        * Quantified predicates apply the inner predicate to each resolved element; empty collections evaluate to true.
    - Rejection/validation rules:
        * Unknown `kind`, `agg`, or `op` values are fatal at invariant load.
        * Missing required keys (`kind`, `op`, `path`, etc.) are fatal.
        * Mixed-type comparisons (e.g., string vs number) are rejected before execution.
        * Nested quantifiers must be well-founded; cycles via `field` references are impossible and need not be detected.
    - Encoded examples:
        * Non-negative balances:
          ```
          { "name": "ledger.balance_nonnegative",
            "predicate": { "kind": "forall", "path": "balances.*", "predicate":
              { "kind": "cmp", "op": "gte", "left": { "kind": "field", "path": "balances.*" }, "right": 0 } },
            "message": "negative balance detected" }
          ```
        * Sum preserved:
          ```
          { "kind": "aggregate", "agg": "sum", "path": "balances.*", "op": "eq", "value": 0 }
          ```
        * Positive transfer amounts:
          ```
          { "kind": "forall", "path": "transfers[*]", "predicate":
            { "kind": "cmp", "op": "gt",
              "left": { "kind": "field", "path": "transfers[*].amount" },
              "right": 0 } }
          ```
    - Binding responsibility:
        * Reject host-language predicates that cannot be lowered to the canonical form.
        * Preserve invariant names and messages verbatim when emitting canonical predicates.
  Invariant file structure:
    - Format: JSON file provided via `--invariants <file>`.
    - Top-level: array of invariant objects `{ "name": <string>, "predicate": <canonical-predicate>, "message": <string> }`.
    - Parsing rules:
        * Unknown fields are rejected with a validation error that lists the offending keys.
        * Missing `name`, `predicate`, or `message` fields are fatal at load time.
        * Duplicate names in the file are rejected before simulation starts.
    - Validation errors:
        * Reported deterministically with file offset/line when available and echoed in CLI output.
        * Engine refuses to start if any invariant fails to parse or validate.
  Observation and invariant examples:
    - Observation payload:
        ```
        {
          "balances": { "alice": 10, "bob": -1 },
          "transfers": [{ "from": "bob", "to": "alice", "amount": 1, "sequence": 42 }],
          "truncated": false
        }
        ```
    - Corresponding invariants:
        * Non-negative balances:
          ```
          { "name": "ledger.balance_nonnegative",
            "predicate": "forall balances.* >= 0",
            "message": "negative balance detected in balances.*" }
          ```
          Failure message example: `negative balance detected in balances.bob: -1`
        * Sequence monotonicity:
          ```
          { "name": "ledger.sequence_monotonic",
            "predicate": "forall transfers[*].sequence is strictly_increasing",
            "message": "transfer sequences must be strictly increasing" }
          ```
          Failure message example: `transfer sequences must be strictly increasing: saw 42 then 40`
        * Sum conservation:
          ```
          { "name": "ledger.sum_preserved",
            "predicate": "sum(balances.*) == 0",
            "message": "ledger sum drifted: expected 0" }
          ```
          Failure message example: `ledger sum drifted: expected 0, saw 9`
    - Observation versioning example:
        * Observation `balances.v1` continues emitting `{ "balances": { ... } }` while new observation `balances.v2` adds `"currency": "USD"`; invariants referencing v1 remain valid during the run, and new invariants can target v2 with names like `ledger_v2.balance_nonnegative`.

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
  Semantics:
    - Targetability by protocol command:
        * crash: may be scheduled against `init`, `apply`, `restore`, or `observe` because all can trigger system-side persistence; forbidden against `shutdown`.
        * io_error: only applies to `apply` (simulated user operations) to model replayable adapter/system IO failures that trigger command replay.
        * delay:<resource>: applies to any command that touches that resource; resources are adapter-defined identifiers (e.g., `storage`, `network`).
    - Step addressing:
        * Steps are scheduler-issued command indices starting at 1 for the first `init`.
        * `delay` duration is measured in logical scheduler steps, not wall-clock time; a `delay:storage@5+2` blocks resource `storage` for steps 5 and 6 and releases before step 7.
    - Ordering and conflicts:
        * Multiple faults on the same step are ordered deterministically by (step, fault type, resource/name) to guarantee stable replay; canonical sort order is crash < io_error < delay and lexicographic within equal types.
        * Conflicting faults targeting the same command (e.g., crash@3 and io_error@3) are applied in canonical order until one makes the command abort; remaining faults for that step are still recorded but may become no-ops if the command never executes.
        * Overlapping delays on the same resource coalesce by taking the maximum end step; delays on distinct resources coexist.
    - Replay and shrinking guarantees:
        * Fault schedules are normalized to canonical ordering before execution and persisted in repros; shrinker preserves ordering and only removes or retimes faults.
        * When shrinking ties (two faults retimed to same step), canonical ordering is re-applied so replay stays byte-identical.
        * Shrink preference order still applies (fewer steps → fewer operations → fewer faults → earlier timing) and never violates determinism.

  Fault schedule examples:
    - Basic schedule:
        * Step 1: init
        * Step 2: apply(opA) with io_error@2
        * Step 3: apply(opB)
        * Step 4: observe
    - Overlapping and normalized schedule:
        * User-specified: crash@5, io_error@5, delay:storage@4+3, delay:network@6+1
        * Normalized execution order:
            - Step 4: delay:storage starts (covers steps 4-6)
            - Step 5: crash then io_error (io_error may be moot if crash prevents completion)
            - Step 6: delay:storage continues; delay:network starts (covers step 6)
        * Shrink behavior example:
            - If shrinker retimes io_error@5 to @4, canonical ordering becomes: delay:storage@4+3, io_error@4, crash@5; replay uses this exact ordering even though faults overlap.

SCHEDULER:
  - Step-based and deterministic
  - Single logical clock
  - No threads, sleeps, or wall-clock time
  - Same seed + config => identical execution
  Semantics:
    - Commands are issued sequentially: init → apply* → (crash/restore pairs) → observe → shutdown; each issuance consumes one step index.
    - Delays pause issuance of commands that target a blocked resource; paused commands are replayed at the next step once all relevant delays expire.
    - Canonical fault ordering is applied per step before execution; when multiple faults affect the same step, scheduler executes them in canonical order and records no-ops explicitly for replay.
    - Shrinker replays using the same scheduler; normalized fault schedules ensure shrink steps map 1:1 to replay steps even when timing ties occur.
  Command replay and protocol timeout handling (per command):
    - `init`: replayable once on `retryable=true` or protocol timeout; the system must return identical initial state. Timeout followed by replay is recorded with `timeout=true` then `replay=true` in trace.
    - `apply`: replayable with bounded attempts; idempotent requirement enforced by invariant comparison between attempts. Command replays after faults such as `io_error@step` are explicitly recorded.
    - `crash`: replayable once on protocol timeout/replayable error; repeated crashes must emit identical persisted `state`. If a crash fault is scheduled, the crash command still must respond with `{ok:true,state}` unless the fault prevents execution, in which case the engine records `fatal=crash_unrecoverable`.
    - `restore`: replayable once; idempotent reconstruction from the provided `state` is mandatory. Protocol timeout escalates to fatal on second occurrence.
    - `observe`: replayable once; must be pure. Faults targeting observe (e.g., crash@step) cause the scheduler to issue crash handling before continuing.
    - `shutdown`: never replayed; protocol timeout is fatal.
  Examples under faults:
    - Protocol timeout on observe: engine marks the step as `timeout`, replays observe once; second timeout => protocol error (exit code 2) with repro entry noting `command=observe`, `timeout_count=2`.
    - Crash during crash command: if a crash fault is injected at the crash step, engine still expects the adapter’s crash response; absence is treated as fatal mismatch and logged.
    - Restore after delayed resources: delays block issuance until released; once restore is issued, a replayable error leads to one command replay with identical `state` payload captured in trace.

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
      * last persisted crash `state` payload if a crash occurred before failure (mirrors the line recorded in trace)
  - `nomercy replay <repro.json>` must reproduce exactly

CLI (PRIMARY INTERFACE):
  nomercy beg <system>
  nomercy pray <system>
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
  Qualification:
    - `beg` performs deterministic validation without issuing simulation commands.
    - `pray` implicitly performs qualification first when no prior success is recorded for the current adapter manifest; failures abort before any simulation run.

  Seed selection and reporting:
    - Default seed is derived deterministically from the adapter manifest hash + engine version (e.g., `seed = siphash(engine_version || manifest_hash)`), ensuring identical seeds for identical inputs when the user omits `--seed`.
    - CLI prints the seed on the first line of output for every command (`seed=<n>`), even when provided explicitly, so operators can copy/paste it into reruns.
    - `replay` and `shrink` refuse `--seed` because seed comes from the repro artifact; CLI surfaces the repro’s seed in the header for confirmation.

  Configuration precedence (highest wins):
    1) CLI flags
    2) Config file passed via `--config <path>`
    3) Environment variables (`NOMERCY_*`)
    - All resolved values are echoed once in deterministic `key=value` lines under a `config:` block; unspecified values are omitted rather than listed as `null`.

  Repro and output layout:
    - All artifacts for a run live under `./target/nomercy/<system>/` (or a workspace-local cache when outside a repo).
    - Fresh failures emit `repro.json` and `trace.json` in that directory; shrink writes `repro.shrunk.json` and `trace.shrunk.json` alongside the originals.
    - Required metadata inside repros:
        * `engine_version`: semantic version of the CLI/engine binary
        * `adapter_manifest_hash`: checksum of `adapter.manifest.json` used for the run
        * `invariant_file_hash`: checksum of the `--invariants` file that was loaded
        * `seed`, `fault_schedule`, and minimal failing trace
    - `replay` reads repros in-place and writes no new files unless `--trace` is set (then `trace.replayed.json` is written next to the repro).
    - All file names and directories are deterministic and referenced verbatim in CLI output to enable copy/paste.

  Exit codes (CI contract):
    - 0: success / invariant satisfied / replay matched
    - 1: invariant failure (signals a real finding; CI should fail and archive repro)
    - 2: protocol error (malformed adapter responses, version mismatch, timeout escalation)
    - 3: adapter build/generation error (failed to compile or validate adapter)
    - 4: system_not_deterministic (qualification failure)
    - Any other code: unexpected engine error (CI treats as infrastructure failure and should rerun)
    - CI guidance: treat 0 as pass, 1 as fail-with-artifact, 2-3 as fail-fast needing investigation; 4 rejects nondeterministic systems; non-listed codes should trigger a rerun then escalation.

  Documentation and CLI guidance (informational):
    - Determinism is a prerequisite; qualification enforces it before any simulation run.
    - Absence of a counterexample only means “not yet found,” not proof of correctness.
    - nomercy explores deterministic state spaces; it is not a test harness.
    - CI usage is limited to replaying known repros; exploratory simulation is expected to run continuously outside CI.

  CLI output format (deterministic, parser-friendly):
    - Output is plain text, not YAML; indentation is two spaces for nested blocks.
    - Headings are unindented identifiers ending with `:` (e.g., `config:`).
    - Entries inside a block are `key=value` pairs indented by two spaces; values never contain newlines.
    - EBNF:
      ```
      output   ::= (heading block?)* status
      heading  ::= IDENT ':' LF
      block    ::= (INDENT entry LF)+
      entry    ::= IDENT '=' VALUE
      status   ::= 'status=' IDENT LF?
      INDENT   ::= '  '
      ```
    - Parsers may safely ignore unknown headings; ordering is stable per command.

  Minimal deterministic CLI output (copy/paste friendly):
    run:
      ```
      seed=1234
      config:
        invariants=spec/ledger_invariants.json
        budget=1000
      adapter=target/nomercy/ledger/nomercy-adapter manifest_hash=9f3b...12
      replay: nomercy replay target/nomercy/ledger/repro.json
      status=ok
      ```

    replay:
      ```
      seed=1234
      repro=target/nomercy/ledger/repro.json
      adapter=target/nomercy/ledger/nomercy-adapter manifest_hash=9f3b...12
      status=ok
      ```

    shrink (after failure):
      ```
      seed=1234
      repro_in=target/nomercy/ledger/repro.json
      repro_out=target/nomercy/ledger/repro.shrunk.json
      trace_out=target/nomercy/ledger/trace.shrunk.json
      adapter_manifest_hash=9f3b...12
      invariant=ledger.balance_nonnegative
      status=ok
      ```

    Failure cases append a single `status=` line with the exit code reason, e.g., `status=invariant_failed` or `status=protocol_error` and always include the repro path when available.

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

COMMON CAUSES OF SYSTEM REJECTION (APPENDIX):
  - Use of unordered collections with iteration-dependent behavior
  - Internal generation of random or time-based identifiers
  - Reading system time or randomness
  - Emitting non-canonical JSON
  - Observation shape instability
  - Floating-point nondeterminism
  - Implicit reliance on exactly-once execution
  - Undeclared side effects or IO

NON-GOALS:
  - Probabilistic testing or fuzzing
  - Thread realism
  - Unit-test replacement
  - Implicit command replays or magic behavior
  - User-written adapters

SLOGAN:
  “Deterministic universes. No mercy.”
