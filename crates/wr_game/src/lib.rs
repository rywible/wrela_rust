#![forbid(unsafe_code)]

use wr_core::{CrateBoundary, CrateEntryPoint, TweakPack};
use wr_ecs::{
    EcsRuntime, GamePlugin, HeadlessActorSpawn, HeadlessScenarioWorld, HeadlessScriptedInput,
};
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

pub fn compose_game_runtime(
    plugins: impl IntoIterator<Item = Box<dyn GamePlugin>>,
) -> Result<EcsRuntime, String> {
    let mut runtime = EcsRuntime::new();
    for plugin in plugins {
        runtime.add_boxed_plugin(plugin)?;
    }
    Ok(runtime)
}

pub fn run_headless_scenario(scenario: &ScenarioRequest) -> HeadlessScenarioSummary {
    run_headless_scenario_with_tweak_pack(scenario, None)
}

pub fn run_headless_scenario_with_tweak_pack(
    scenario: &ScenarioRequest,
    tweak_pack: Option<&TweakPack>,
) -> HeadlessScenarioSummary {
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
    if let Some(pack) = tweak_pack
        && let Err(error) = world.apply_tweak_pack(pack)
    {
        return failed_summary(
            scenario,
            0,
            0,
            Vec::new(),
            format!("failed to apply tweak pack: {error}"),
        );
    }
    let mut assertions = Vec::new();

    for frame in 0..scenario.fixed_steps {
        world.apply_inputs(scripted_inputs.iter().filter(|input| input.frame == frame));
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
            Some(vec!["Headless execution uses the ECS schedule spine.".to_owned()]),
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

#[cfg(test)]
mod tests {
    use super::*;
    use wr_telemetry::SeedInfo;

    fn test_scenario(assertions: Vec<ScenarioAssertion>) -> ScenarioRequest {
        ScenarioRequest {
            schema_version: wr_tools_harness::HARNESS_SCHEMA_VERSION.to_owned(),
            scenario_path: "scenarios/smoke/startup.ron".to_owned(),
            tweak_pack_path: None,
            simulation_rate_hz: 60,
            fixed_steps: 1,
            seed: SeedInfo::new("hero_forest", "0xDEADBEEF"),
            spawned_actors: vec![wr_tools_harness::ScenarioActorSpawn {
                actor_id: "player".to_owned(),
                actor_kind: "player_sword".to_owned(),
                seed_stream: Some("player".to_owned()),
            }],
            scripted_inputs: Vec::new(),
            assertions,
        }
    }

    fn test_world() -> HeadlessScenarioWorld {
        HeadlessScenarioWorld::new(
            60,
            RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse"),
            &[HeadlessActorSpawn {
                actor_id: "player".to_owned(),
                actor_kind: "player_sword".to_owned(),
                seed_stream: Some("player".to_owned()),
            }],
        )
    }

    fn test_assertion(
        metric: &str,
        comparator: &str,
        expected: f32,
        tolerance: Option<f32>,
    ) -> ScenarioAssertion {
        ScenarioAssertion {
            frame: Some(0),
            metric: metric.to_owned(),
            comparator: comparator.to_owned(),
            expected,
            tolerance,
        }
    }

    #[test]
    fn compare_metric_supports_declared_comparators() {
        let cases = [
            ("eq", 1.05, 1.0, Some(0.05), true),
            ("ne", 1.2, 1.0, Some(0.1), true),
            ("gt", 2.0, 1.0, None, true),
            ("gte", 1.0, 1.0, None, true),
            ("lt", 0.5, 1.0, None, true),
            ("lte", 1.0, 1.0, None, true),
            ("eq", 1.2, 1.0, Some(0.05), false),
        ];

        for (comparator, actual, expected, tolerance, passed) in cases {
            let assertion =
                test_assertion("world.frames_simulated", comparator, expected, tolerance);
            let outcome = compare_metric(actual, &assertion, 0);

            assert_eq!(outcome.passed, passed, "comparator {comparator} should match expectation");

            if passed {
                assert_eq!(
                    outcome.details, None,
                    "passing comparator {comparator} should not report details"
                );
            } else {
                assert!(
                    outcome.details.as_deref().is_some_and(
                        |details| details.contains("expected to equal 1 within tolerance 0.05")
                    ),
                    "failing comparator {comparator} should explain the mismatch"
                );
            }
        }
    }

    #[test]
    fn evaluate_assertions_stops_at_first_failure() {
        let mut world = test_world();
        world.step(0);

        let assertions = [
            test_assertion("world.actor_count", "eq", 1.0, Some(0.0)),
            test_assertion("world.frames_simulated", "eq", 2.0, Some(0.0)),
            test_assertion("world.actor_count", "eq", 1.0, Some(0.0)),
        ];
        let mut records = Vec::new();

        let outcome = evaluate_assertions(&world, assertions.iter(), &mut records);

        assert!(matches!(outcome, AssertionEvaluation::Failed(_)));
        assert_eq!(records.len(), 2, "evaluation should stop after the first failure");
        assert!(records[0].passed, "the first assertion should pass");
        assert!(!records[1].passed, "the second assertion should fail");
        assert_eq!(records[1].metric, "world.frames_simulated");
        assert!(
            records[1].details.as_deref().is_some_and(|details| details.contains("observed 1"))
        );
    }

    #[test]
    fn run_headless_scenario_reports_bad_seed_before_simulation() {
        let scenario = test_scenario(Vec::new());
        let scenario =
            ScenarioRequest { seed: SeedInfo::new("hero_forest", "not-a-hex-seed"), ..scenario };

        let summary = run_headless_scenario(&scenario);

        assert_eq!(summary.result.status, HarnessStatus::Failed);
        assert_eq!(summary.metrics.frames_simulated, 0);
        assert_eq!(summary.metrics.applied_input_count, 0);
        assert!(
            summary
                .result
                .details
                .as_deref()
                .is_some_and(|details| details.contains("failed to parse root seed"))
        );
        assert!(summary.notes.as_ref().is_some_and(|notes| {
            notes
                .iter()
                .any(|note| note.contains("failed before the fixed-step simulation completed"))
        }));
    }

    #[test]
    fn run_headless_scenario_with_tweak_pack_surfaces_tweak_metrics() {
        let scenario = test_scenario(vec![test_assertion(
            "tweaks.dirty_namespace_count",
            "eq",
            1.0,
            Some(0.0),
        )]);
        let pack = TweakPack::new(std::collections::BTreeMap::from([(
            "world.wind_strength".to_owned(),
            wr_core::TweakValue::Scalar(0.5),
        )]));

        let summary = run_headless_scenario_with_tweak_pack(&scenario, Some(&pack));

        assert_eq!(summary.result.status, HarnessStatus::Passed);
        assert_eq!(summary.assertions.len(), 1);
        assert!(summary.assertions[0].passed);
    }
}
