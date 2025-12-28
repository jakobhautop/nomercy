//! nomercy-core: deterministic adversarial simulation engine primitives.
//!
//! This crate defines the foundational types and behaviors used by the NoMercy
//! engine. It is intentionally lightweight to enable rapid iteration on the
//! CLI and the reference mock project while staying close to the specification.

use serde::{Deserialize, Serialize};

/// Configuration for an engine run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EngineConfig<C> {
    /// Deterministic seed derived from the adapter manifest or provided by the user.
    pub seed: u64,
    /// Optional logical step budget for a run.
    pub budget: Option<u64>,
    /// Binding-provided system configuration.
    pub system_config: C,
}

/// Snapshot of a single step in a schedule.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationStep<Op, Observation> {
    pub index: usize,
    pub operation: Op,
    pub observation: Observation,
}

/// Outcome of a deterministic schedule execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationOutcome<S>
where
    S: SimulationSystem,
{
    pub config: EngineConfig<S::Config>,
    pub steps: Vec<SimulationStep<S::Operation, S::Observation>>,
    pub crash_state: S::PersistedState,
    pub post_crash_observation: S::Observation,
}

/// Trait implemented by systems that can be simulated by NoMercy.
///
/// The trait mirrors the capability model defined in the specification.
pub trait SimulationSystem {
    type Config: Clone;
    type Operation: Clone;
    type Observation: Clone;
    type PersistedState: Clone;

    /// Initialize a fresh system using the provided configuration.
    fn init(config: Self::Config) -> Self;

    /// Apply a single operation to the system.
    fn apply(&mut self, op: Self::Operation);

    /// Crash the system and return persisted state.
    fn crash(self) -> Self::PersistedState;

    /// Restore a system from persisted state.
    fn restore(state: Self::PersistedState) -> Self;

    /// Capture a deterministic observation of the current state.
    fn observe(&self) -> Self::Observation;
}

/// Run a single deterministic schedule against a system.
///
/// This helper mirrors the command lifecycle at a high level:
/// init -> apply* -> crash -> restore -> observe.
pub fn simulate<S>(
    config: EngineConfig<S::Config>,
    operations: &[S::Operation],
) -> SimulationOutcome<S>
where
    S: SimulationSystem,
{
    let mut system = S::init(config.system_config.clone());
    let mut steps = Vec::with_capacity(operations.len());

    for (index, op) in operations.iter().cloned().enumerate() {
        system.apply(op.clone());
        let observation = system.observe();
        steps.push(SimulationStep {
            index: index + 1,
            operation: op,
            observation,
        });
    }

    let crash_state = system.crash();
    let restored = S::restore(crash_state.clone());
    let post_crash_observation = restored.observe();

    SimulationOutcome {
        config,
        steps,
        crash_state,
        post_crash_observation,
    }
}
