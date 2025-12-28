use flake::{run_flake_schedule, FlakeOp, FlakeState};

#[test]
fn crash_restore_round_trip_is_deterministic() {
    let operations = vec![
        FlakeOp::Increment(2),
        FlakeOp::Decrement(1),
        FlakeOp::Reset(5),
    ];
    let outcome = run_flake_schedule(42, Some(10), &operations);

    assert_eq!(outcome.steps.len(), operations.len());
    assert_eq!(
        outcome
            .steps
            .iter()
            .map(|step| step.observation.counter)
            .collect::<Vec<_>>(),
        vec![2, 1, 5]
    );

    let expected_state = FlakeState {
        counter: 5,
        journal: operations,
    };

    assert_eq!(outcome.crash_state, expected_state);
    assert_eq!(outcome.post_crash_observation.counter, 5);
    assert_eq!(
        outcome.post_crash_observation.applied,
        expected_state.journal
    );
}
