# FlakySessions

FlakySessions is a **minimal session server** used to demonstrate why
crash-safe, replay-safe systems are harder than they look.

The code is intentionally small and intentionally naive.

There are no clocks, no randomness, and no concurrency.
Despite that, the system still contains real bugs.

---

## Purpose

FlakySessions exists to answer one question:

> **“Is this session ID valid right now?”**

It supports:
- creating sessions for users
- revoking sessions
- validating session IDs

The implementation looks reasonable and would pass most unit tests.

FlakySessions is not meant to be realistic or production-ready.
It is meant to be **understandable** — and still wrong.

---

## Invariants

These are **semantic guarantees** the system is expected to uphold.
They are not implementation details.

### 1. Revoked means invalid forever

Once a session is revoked, it must never become valid again.

There is no refresh.
There is no reactivation.
Revocation is permanent.

---

### 2. Validation is deterministic

Given the same history of operations,
validating a session ID must always return the same result.

No operation may depend on:
- time
- randomness
- retries
- partial execution

---

### 3. No active session after revoke

If a session has been revoked, it must not appear as active
in any observation of system state.

---

## Bugs We Expect to Find

The current implementation contains bugs that only appear when:
- operations are replayed
- crashes occur mid-operation
- state is restored from partial persistence

Examples of real failure modes:

### Session resurrection

A revoked session may become active again after:
- a crash during `revoke`
- a replay of a previously issued operation
- a restore followed by validation

---

### Duplicate effects from replay

If `create` is replayed:
- multiple sessions may be created unintentionally
- invariants about revocation and validity can be violated
