#![forbid(unsafe_code)]

use wr_core::{CrateBoundary, CrateEntryPoint};
use wr_ecs::{HeadlessActorSpawn, HeadlessScenarioWorld, HeadlessScriptedInput};
use wr_tools_harness::{
    FailureKind, HarnessStatus, ResultEnvelope, SUPPORTED_ASSERTION_COMPARATORS, ScenarioAssertion,
    ScenarioAssertionResult, ScenarioExecutionMetrics, ScenarioRequest,
};
use wr_world_seed::{RootSeed, stable_hash_hex};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_game", CrateBoundary::Composition, true)
}

pub const fn scaffold_members() -> [CrateEntryPoint; 21] {
    [
        wr_core::init_entrypoint(),
        wr_math::init_entrypoint(),
        wr_world_seed::init_entrypoint(),
        wr_ecs::init_entrypoint(),
        wr_platform::init_entrypoint(),
        wr_render_api::init_entrypoint(),
        wr_render_wgpu::init_entrypoint(),
        wr_render_atmo::init_entrypoint(),
        wr_render_scene::init_entrypoint(),
        wr_render_post::init_entrypoint(),
        wr_world_gen::init_entrypoint(),
        wr_procgeo::init_entrypoint(),
        wr_physics::init_entrypoint(),
        wr_combat::init_entrypoint(),
        wr_ai::init_entrypoint(),
        wr_actor_player::init_entrypoint(),
        wr_actor_wraith::init_entrypoint(),
        wr_vfx::init_entrypoint(),
        wr_tools_ui::init_entrypoint(),
        wr_tools_harness::init_entrypoint(),
        wr_telemetry::init_entrypoint(),
    ]
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeadlessScenarioSummary {
    pub result: ResultEnvelope,
    pub metrics: ScenarioExecutionMetrics,
    pub assertions: Vec<ScenarioAssertionResult>,
    pub determinism_hash: String,
    pub notes: Option<Vec<String>>,
}

pub fn run_headless_scenario(scenario: &ScenarioRequest) -> HeadlessScenarioSummary {
    let seed = match RootSeed::parse_hex(&scenario.seed.value_hex) {
        Ok(seed) => seed,
        Err(error) => {
            return failed_summary(
                scenario,
                0,
                0,
                Vec::new(),
                format!("failed to parse root seed: {error}"),
            );
        }
    };

    let actor_spawns = scenario
        .spawned_actors
        .iter()
        .map(|actor| HeadlessActorSpawn {
            actor_id: actor.actor_id.clone(),
            actor_kind: actor.actor_kind.clone(),
            seed_stream: actor.seed_stream.clone(),
        })
        .collect::<Vec<_>>();
    let scripted_inputs = scenario
        .scripted_inputs
        .iter()
        .map(|input| HeadlessScriptedInput {
            frame: input.frame,
            action: input.action.clone(),
            state: input.state.clone(),
        })
        .collect::<Vec<_>>();

    let mut world = HeadlessScenarioWorld::new(scenario.simulation_rate_hz, seed, &actor_spawns);
    let mut assertions = Vec::new();

    for frame in 0..scenario.fixed_steps {
        world.apply_inputs(frame, scripted_inputs.iter().filter(|input| input.frame == frame));
        world.step(frame);

        let due_assertions =
            scenario.assertions.iter().filter(|assertion| assertion.frame == Some(frame));

        match evaluate_assertions(&world, due_assertions, &mut assertions) {
            AssertionEvaluation::Continue => {}
            AssertionEvaluation::Failed(details) => {
                return finalize_summary(
                    scenario,
                    &world,
                    assertions,
                    ResultEnvelope {
                        status: HarnessStatus::Failed,
                        summary: format!(
                            "Scenario failed on frame {} after {} simulated steps.",
                            frame,
                            world.frames_simulated()
                        ),
                        failure_kind: Some(FailureKind::ScenarioFailed),
                        details: Some(details),
                    },
                    Some(vec!["Execution stopped at the first failing assertion.".to_owned()]),
                );
            }
        }
    }

    let final_assertions = scenario.assertions.iter().filter(|assertion| assertion.frame.is_none());

    match evaluate_assertions(&world, final_assertions, &mut assertions) {
        AssertionEvaluation::Continue => finalize_summary(
            scenario,
            &world,
            assertions,
            ResultEnvelope {
                status: HarnessStatus::Passed,
                summary: format!(
                    "Scenario completed {} fixed steps without assertion failures.",
                    world.frames_simulated()
                ),
                failure_kind: None,
                details: None,
            },
            Some(vec![
                "Headless execution uses the bootstrap fixed-step world; ECS scheduling lands in PR-007."
                    .to_owned(),
            ]),
        ),
        AssertionEvaluation::Failed(details) => finalize_summary(
            scenario,
            &world,
            assertions,
            ResultEnvelope {
                status: HarnessStatus::Failed,
                summary: format!(
                    "Scenario failed after {} simulated steps.",
                    world.frames_simulated()
                ),
                failure_kind: Some(FailureKind::ScenarioFailed),
                details: Some(details),
            },
            Some(vec!["Execution stopped at the first failing assertion.".to_owned()]),
        ),
    }
}

enum AssertionEvaluation {
    Continue,
    Failed(String),
}

fn evaluate_assertions<'a>(
    world: &HeadlessScenarioWorld,
    assertions: impl IntoIterator<Item = &'a ScenarioAssertion>,
    records: &mut Vec<ScenarioAssertionResult>,
) -> AssertionEvaluation {
    for assertion in assertions {
        let frame = assertion.frame.unwrap_or_else(|| world.frames_simulated().saturating_sub(1));
        let actual = world.metric_value(&assertion.metric);
        let outcome = match actual {
            Some(actual) => compare_metric(actual, assertion, frame),
            None => ScenarioAssertionResult {
                metric: assertion.metric.clone(),
                comparator: assertion.comparator.clone(),
                frame,
                expected: assertion.expected,
                actual: None,
                tolerance: assertion.tolerance,
                passed: false,
                details: Some(format!("metric `{}` is not available", assertion.metric)),
            },
        };

        let passed = outcome.passed;
        let details = outcome.details.clone();
        records.push(outcome);

        if !passed {
            return AssertionEvaluation::Failed(details.unwrap_or_else(|| {
                format!("assertion `{}` failed without a detailed message", assertion.metric)
            }));
        }
    }

    AssertionEvaluation::Continue
}

fn compare_metric(
    actual: f32,
    assertion: &ScenarioAssertion,
    frame: u32,
) -> ScenarioAssertionResult {
    let tolerance = assertion.tolerance.unwrap_or(0.0);
    let difference = (actual - assertion.expected).abs();
    let comparator = assertion.comparator.as_str();
    let passed = match comparator {
        "eq" => difference <= tolerance,
        "ne" => difference > tolerance,
        "gt" => actual > assertion.expected,
        "gte" => actual >= assertion.expected,
        "lt" => actual < assertion.expected,
        "lte" => actual <= assertion.expected,
        _ => false,
    };

    let details = if passed {
        None
    } else if SUPPORTED_ASSERTION_COMPARATORS.contains(&comparator) {
        let expectation = match comparator {
            "eq" => format!("equal {} within tolerance {}", assertion.expected, tolerance),
            "ne" => {
                format!("differ from {} by more than tolerance {}", assertion.expected, tolerance)
            }
            "gt" => format!("be greater than {}", assertion.expected),
            "gte" => format!("be greater than or equal to {}", assertion.expected),
            "lt" => format!("be less than {}", assertion.expected),
            "lte" => format!("be less than or equal to {}", assertion.expected),
            _ => unreachable!("supported comparators are matched above"),
        };

        Some(format!(
            "metric `{}` at frame {} expected to {} but observed {}",
            assertion.metric, frame, expectation, actual
        ))
    } else {
        Some(format!("unsupported comparator `{}`", assertion.comparator))
    };

    ScenarioAssertionResult {
        metric: assertion.metric.clone(),
        comparator: assertion.comparator.clone(),
        frame,
        expected: assertion.expected,
        actual: Some(actual),
        tolerance: assertion.tolerance,
        passed,
        details,
    }
}

fn failed_summary(
    scenario: &ScenarioRequest,
    frames_simulated: u32,
    applied_input_count: u32,
    assertions: Vec<ScenarioAssertionResult>,
    details: String,
) -> HeadlessScenarioSummary {
    let metrics = ScenarioExecutionMetrics {
        frames_requested: scenario.fixed_steps,
        frames_simulated,
        simulation_rate_hz: scenario.simulation_rate_hz,
        spawned_actor_count: scenario.spawned_actors.len() as u32,
        scripted_input_count: scenario.scripted_inputs.len() as u32,
        applied_input_count,
    };
    let determinism_hash = stable_hash_hex([
        scenario.seed.value_hex.as_bytes(),
        details.as_bytes(),
        metrics.frames_simulated.to_string().as_bytes(),
    ]);

    HeadlessScenarioSummary {
        result: ResultEnvelope {
            status: HarnessStatus::Failed,
            summary: "Scenario could not be executed.".to_owned(),
            failure_kind: Some(FailureKind::ScenarioFailed),
            details: Some(details),
        },
        metrics,
        assertions,
        determinism_hash,
        notes: Some(vec![
            "Execution failed before the fixed-step simulation completed.".to_owned(),
        ]),
    }
}

fn finalize_summary(
    scenario: &ScenarioRequest,
    world: &HeadlessScenarioWorld,
    assertions: Vec<ScenarioAssertionResult>,
    result: ResultEnvelope,
    notes: Option<Vec<String>>,
) -> HeadlessScenarioSummary {
    let metrics = ScenarioExecutionMetrics {
        frames_requested: scenario.fixed_steps,
        frames_simulated: world.frames_simulated(),
        simulation_rate_hz: world.simulation_rate_hz(),
        spawned_actor_count: world.actor_count(),
        scripted_input_count: scenario.scripted_inputs.len() as u32,
        applied_input_count: world.applied_input_count(),
    };
    let determinism_hash = stable_hash_hex(
        world
            .deterministic_records()
            .into_iter()
            .chain(assertions.iter().map(|assertion| {
                format!(
                    "assert:{}:{}:{}:{:?}:{}",
                    assertion.metric,
                    assertion.comparator,
                    assertion.frame,
                    assertion.actual,
                    assertion.passed
                )
            }))
            .chain([
                format!("seed={}", scenario.seed.value_hex),
                format!("requested_steps={}", scenario.fixed_steps),
            ])
            .map(|record| record.into_bytes()),
    );

    HeadlessScenarioSummary { result, metrics, assertions, determinism_hash, notes }
}
