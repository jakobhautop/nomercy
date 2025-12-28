//! Flake: a deterministic mock system used for exercising NoMercy end-to-end.
//!
//! The implementation is intentionally small but mirrors the lifecycle expected
//! by the engine:
//! - init(config) -> state
//! - apply(op) -> state
//! - crash() -> persisted state
//! - restore(persisted_state) -> state
//! - observe() -> observation

use nomercy_core::{simulate, EngineConfig, SimulationSystem};
use serde::{Deserialize, Serialize};

/// Configuration for the Flake system.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlakeConfig {
    pub initial_counter: i64,
}

impl Default for FlakeConfig {
    fn default() -> Self {
        Self { initial_counter: 0 }
    }
}

/// Operations supported by Flake.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum FlakeOp {
    Increment(i64),
    Decrement(i64),
    Reset(i64),
}

/// Observations emitted by Flake after each operation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FlakeObservation {
    pub counter: i64,
    pub applied: Vec<FlakeOp>,
}

/// Persisted state captured on crash.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FlakeState {
    pub counter: i64,
    pub journal: Vec<FlakeOp>,
}

/// Deterministic reference implementation of the mock system.
#[derive(Clone, Debug)]
pub struct Flake {
    counter: i64,
    journal: Vec<FlakeOp>,
}

impl SimulationSystem for Flake {
    type Config = FlakeConfig;
    type Operation = FlakeOp;
    type Observation = FlakeObservation;
    type PersistedState = FlakeState;

    fn init(config: Self::Config) -> Self {
        Flake {
            counter: config.initial_counter,
            journal: Vec::new(),
        }
    }

    fn apply(&mut self, op: Self::Operation) {
        match op {
            FlakeOp::Increment(amount) => self.counter += amount,
            FlakeOp::Decrement(amount) => self.counter -= amount,
            FlakeOp::Reset(value) => self.counter = value,
        }

        self.journal.push(op);
    }

    fn crash(self) -> Self::PersistedState {
        FlakeState {
            counter: self.counter,
            journal: self.journal,
        }
    }

    fn restore(state: Self::PersistedState) -> Self {
        Flake {
            counter: state.counter,
            journal: state.journal,
        }
    }

    fn observe(&self) -> Self::Observation {
        FlakeObservation {
            counter: self.counter,
            applied: self.journal.clone(),
        }
    }
}

/// Convenience helper for running a simple deterministic schedule against Flake.
pub fn run_flake_schedule(
    seed: u64,
    budget: Option<u64>,
    operations: &[FlakeOp],
) -> nomercy_core::SimulationOutcome<Flake> {
    let config = EngineConfig {
        seed,
        budget,
        system_config: FlakeConfig::default(),
    };

    simulate::<Flake>(config, operations)
}
