# nomercy — Rust Developer Guide

This guide explains how to use **nomercy** from Rust to find real bugs in
stateful systems using **deterministic simulation**.

You write normal Rust.
nomercy adds a small amount of structure via attributes and derives.

---

## What nomercy Is

nomercy explores **all the ways your system can break** under:
- crashes
- replays
- partial execution
- restore

You do **not** write tests.
You do **not** write scenarios.
You write **laws** (invariants), and nomercy searches for counterexamples.

If no counterexample is found, it means:
> “Not yet found” — nothing more.

---

## Mental Model (Read This Once)

> You write a deterministic state machine.  
> You describe what must *never* be false.  
> nomercy controls execution, order, crashes, and replay.

Macros exist only to **mark intent**, not to change how you think.

---

## Step 1 — Write Your System (Normal Rust)

You write your system as ordinary Rust code.

Example: a minimal session server.

```rust
use std::collections::BTreeMap;

type SessionId = String;

#[derive(Clone)]
struct Session {
    user: String,
    active: bool,
}

struct State {
    sessions: BTreeMap<SessionId, Session>,
    next_id: u64,
}

impl State {
    fn new() -> Self {
        Self {
            sessions: BTreeMap::new(),
            next_id: 0,
        }
    }

    fn create(&mut self, user: String) -> SessionId {
        let id = format!("s{}", self.next_id);
        self.next_id += 1;
        self.sessions.insert(id.clone(), Session { user, active: true });
        id
    }

    fn revoke(&mut self, id: &str) {
        if let Some(s) = self.sessions.get_mut(id) {
            s.active = false;
        }
    }
}
```

At this point, this is just Rust.
You could ship this code without nomercy.

---

## Step 2 — Mark the System and Operations (Macros)

nomercy uses attribute macros to understand:
- what the system is
- which functions are operations
- which function produces observations

These macros do not change semantics.
They exist so nomercy can generate adapters automatically.

Example:

```rust
use nomercy::prelude::*;

#[system]
struct State { /* … */ }

#[op]
fn create(state: &mut State, user: String) -> SessionId { /* … */ }

#[op]
fn revoke(state: &mut State, id: &str) { /* … */ }
```

Think of these macros as:
“This is important — nomercy should look here.”
Nothing more.

---

## Step 3 — Write Invariants (Pure Rust)

An invariant is a Rust function that must always return true.
It is checked:
- after every operation
- after every crash
- after every restore

Example:

```rust
#[invariant(name = "revoked_never_becomes_active")]
fn revoked_never_becomes_active(state: &State) -> bool {
    state.sessions.values().all(|s| !s.active)
}
```

Rules:
- must be deterministic
- no time
- no randomness
- no IO
- no mutation

If an invariant returns false, nomercy immediately records a repro.

---

## Step 4 — (Optional) Define an Observation Boundary

You do not need this by default.
By default, invariants see your system state directly.

Only introduce an observation if:
- internal state contains caches or helpers
- invariants should see a simplified or stable view
- crashes should erase internal bookkeeping

Example (optional):

```rust
struct Observation {
    sessions: BTreeMap<SessionId, bool>,
}

#[observe]
fn observe(state: &State) -> Observation {
    Observation {
        sessions: state.sessions
            .iter()
            .map(|(id, s)| (id.clone(), s.active))
            .collect(),
    }
}
```

Invariants can now target &Observation instead of &State.
If this feels like extra work, don’t do it.

---

## Step 5 — Operation Order Is Automatic

You never specify:
- which operation runs next
- how many times it runs
- what order operations appear in

nomercy derives the operation space automatically from `#[op]` functions.

Argument values come from deterministic domains:
- `bool` → `{true, false}`
- `strings` → small canonical set
- IDs → values already in state
- enums → all variants
- `Option<T>` → `None | Some(T)`

If an operation should only be legal sometimes, encode that in the code
(e.g. early return or `assume!`).

You never write an `available_ops` function unless you really want to.

---

## Step 6 — Run nomercy

Determinism qualification:

```
nomercy beg my_system
```

Simulation:

```
nomercy pray my_system
```

If a counterexample is found:

```
invariant=revoked_never_becomes_active
repro=target/nomercy/my_system/repro.json
status=invariant_failed
```

---

## Step 7 — Replay and Shrink

Replay exactly:

```
nomercy replay target/nomercy/my_system/repro.json
```

Shrink to a minimal failure:

```
nomercy shrink target/nomercy/my_system/trace.json
```

Shrinking is automatic and deterministic.

---

## What You Never Write

You never write:
- test cases
- operation sequences
- retry logic
- crash handling
- restore logic
- scheduling code

Macros mark what exists.
nomercy decides how it executes.

---

## Design Principles (Explicit)

- Macros mark intent, not behavior
- The default path requires no framework-only types
- Invariants are laws, not tests
- Order, time, and crashes belong to nomercy
- Nondeterminism is rejected up front

---

## Summary

- Write normal Rust
- Add small, explicit macros to mark systems and ops
- Write invariants as pure functions
- Let nomercy explore executions
- Fix counterexamples, not tests
- Deterministic universes. No mercy.
