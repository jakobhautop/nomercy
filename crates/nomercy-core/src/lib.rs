pub mod prelude {
    pub use crate::invariant::{Invariant, InvariantResult};
    pub use crate::operation::Operation;
    pub use crate::simulation::{Simulation, SimulationOutcome, SimulationStatus, SimulationStep};
    pub use crate::system::{Observation, SystemModel};
    pub use nomercy_macros::{invariant, observe, op, system};
}

pub mod invariant {
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    pub struct InvariantResult {
        pub name: &'static str,
        pub holds: bool,
    }

    #[derive(Clone)]
    pub struct Invariant<O> {
        pub name: &'static str,
        check: fn(&O) -> bool,
    }

    impl<O> Invariant<O> {
        pub fn new(name: &'static str, check: fn(&O) -> bool) -> Self {
            Self { name, check }
        }

        pub fn evaluate(&self, observation: &O) -> InvariantResult {
            InvariantResult {
                name: self.name,
                holds: (self.check)(observation),
            }
        }
    }
}

pub mod operation {
    pub struct Operation<S> {
        pub name: &'static str,
        apply: Box<dyn Fn(&mut S) + Send + Sync>,
    }

    impl<S> Operation<S> {
        pub fn new(name: &'static str, apply: impl Fn(&mut S) + Send + Sync + 'static) -> Self {
            Self {
                name,
                apply: Box::new(apply),
            }
        }

        pub fn apply(&self, state: &mut S) {
            (self.apply)(state);
        }
    }
}

pub mod system {
    use crate::{invariant::Invariant, operation::Operation};

    pub struct Observation<S, O> {
        project: Box<dyn Fn(&S) -> O + Send + Sync>,
    }

    impl<S: Clone + 'static> Observation<S, S> {
        pub fn identity() -> Self {
            Observation {
                project: Box::new(Clone::clone),
            }
        }
    }

    impl<S, O> Observation<S, O> {
        pub fn project(project: impl Fn(&S) -> O + Send + Sync + 'static) -> Self {
            Observation {
                project: Box::new(project),
            }
        }

        pub fn view(&self, state: &S) -> O {
            (self.project)(state)
        }
    }

    pub struct SystemModel<S, O> {
        pub name: String,
        pub init: fn() -> S,
        pub observe: Observation<S, O>,
        pub operations: Vec<Operation<S>>,
        pub invariants: Vec<Invariant<O>>,
    }

    impl<S: Clone + 'static> SystemModel<S, S> {
        pub fn new(name: impl Into<String>, init: fn() -> S) -> Self {
            Self {
                name: name.into(),
                init,
                observe: Observation::identity(),
                operations: Vec::new(),
                invariants: Vec::new(),
            }
        }

        pub fn with_observer<O>(
            self,
            observe: impl Fn(&S) -> O + Send + Sync + 'static,
        ) -> SystemModel<S, O> {
            SystemModel {
                name: self.name,
                init: self.init,
                observe: Observation::project(observe),
                operations: self.operations,
                invariants: Vec::new(),
            }
        }
    }

    impl<S: Clone, O> SystemModel<S, O> {
        pub fn operation(mut self, operation: Operation<S>) -> Self {
            self.operations.push(operation);
            self
        }

        pub fn invariant(mut self, invariant: Invariant<O>) -> Self {
            self.invariants.push(invariant);
            self
        }
    }
}

pub mod simulation {
    use serde::Serialize;
    use serde_json::Value;

    use crate::{
        invariant::{Invariant, InvariantResult},
        system::{Observation, SystemModel},
    };

    #[derive(Debug, Serialize, Clone)]
    pub struct SimulationStep {
        pub op: &'static str,
        pub iteration: usize,
    }

    #[derive(Debug, Serialize)]
    pub struct InvariantFailure {
        pub invariant: &'static str,
        pub step: Option<SimulationStep>,
    }

    #[derive(Debug, Serialize)]
    #[serde(tag = "status", rename_all = "snake_case")]
    pub enum SimulationStatus {
        Completed,
        InvariantViolated(InvariantFailure),
    }

    #[derive(Debug, Serialize)]
    pub struct SimulationOutcome {
        pub system: String,
        pub steps: Vec<SimulationStep>,
        pub status: SimulationStatus,
    }

    impl SimulationOutcome {
        pub fn to_json(&self) -> Value {
            serde_json::to_value(self).expect("serialize simulation outcome")
        }
    }

    pub struct Simulation<S, O> {
        model: SystemModel<S, O>,
    }

    impl<S: Clone, O> Simulation<S, O> {
        pub fn new(model: SystemModel<S, O>) -> Self {
            Self { model }
        }

        pub fn run(&self, rounds: usize) -> SimulationOutcome {
            let mut state = (self.model.init)();
            let mut steps = Vec::new();

            if let Some(failure) =
                check_invariants(None, &self.model.invariants, &self.model.observe, &state)
            {
                return SimulationOutcome {
                    system: self.model.name.clone(),
                    steps,
                    status: SimulationStatus::InvariantViolated(failure),
                };
            }

            for iteration in 0..rounds {
                for op in &self.model.operations {
                    op.apply(&mut state);
                    let step = SimulationStep {
                        op: op.name,
                        iteration,
                    };
                    if let Some(failure) = check_invariants(
                        Some(step.clone()),
                        &self.model.invariants,
                        &self.model.observe,
                        &state,
                    ) {
                        return SimulationOutcome {
                            system: self.model.name.clone(),
                            steps,
                            status: SimulationStatus::InvariantViolated(failure),
                        };
                    }
                    steps.push(step);
                }
            }

            SimulationOutcome {
                system: self.model.name.clone(),
                steps,
                status: SimulationStatus::Completed,
            }
        }
    }

    fn check_invariants<S: Clone, O>(
        step: Option<SimulationStep>,
        invariants: &[Invariant<O>],
        observer: &Observation<S, O>,
        state: &S,
    ) -> Option<InvariantFailure> {
        let observation = observer.view(state);

        invariants
            .iter()
            .map(|invariant| invariant.evaluate(&observation))
            .find(|result| !result.holds)
            .map(|InvariantResult { name, .. }| InvariantFailure {
                invariant: name,
                step,
            })
    }
}
