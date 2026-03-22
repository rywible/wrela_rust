use std::collections::BTreeMap;

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use wr_telemetry::{RunMetadata, SeedInfo};

pub const HARNESS_SCHEMA_VERSION: &str = "wr_harness/v1";

#[derive(Debug)]
pub enum HarnessError {
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidPath { path: String },
}

impl HarnessError {
    pub fn invalid_path(path: String) -> Self {
        Self::InvalidPath { path }
    }
}

impl std::fmt::Display for HarnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "i/o error: {error}"),
            Self::Json(error) => write!(f, "json serialization error: {error}"),
            Self::InvalidPath { path } => write!(f, "invalid artifact path: {path}"),
        }
    }
}

impl std::error::Error for HarnessError {}

impl From<std::io::Error> for HarnessError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for HarnessError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    BuildFailed,
    TestFailed,
    ScenarioFailed,
    PerfRegressed,
    VisualRegressed,
    RuntimeCrash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HarnessStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ResultEnvelope {
    pub status: HarnessStatus,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_kind: Option<FailureKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ArtifactDescriptor {
    pub role: String,
    pub path: String,
    pub media_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ScriptedInput {
    pub frame: u32,
    pub action: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ScenarioAssertion {
    pub metric: String,
    pub comparator: String,
    pub expected: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ScenarioRequest {
    pub schema_version: String,
    pub scenario_path: String,
    pub simulation_rate_hz: u32,
    pub fixed_steps: u32,
    pub seed: SeedInfo,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scripted_inputs: Vec<ScriptedInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assertions: Vec<ScenarioAssertion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CaptureRequest {
    pub schema_version: String,
    pub scenario_path: String,
    pub camera_set: String,
    pub frame_count: u32,
    pub seed: SeedInfo,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requested_outputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LookdevVariant {
    pub variant_id: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub overrides: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LookdevSweepRequest {
    pub schema_version: String,
    pub tweak_pack_path: String,
    pub camera_set: String,
    pub seed: SeedInfo,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<LookdevVariant>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DuelMetrics {
    pub duration_ms: u64,
    pub clash_count: u32,
    pub player_hits: u32,
    pub enemy_hits: u32,
    pub average_reengage_time_ms: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DuelReport {
    pub schema_version: String,
    pub metadata: RunMetadata,
    pub seed: SeedInfo,
    pub scenario_path: String,
    pub result: ResultEnvelope,
    pub metrics: DuelMetrics,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<ArtifactDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PerformanceMetrics {
    pub average_frame_ms: f32,
    pub p95_frame_ms: f32,
    pub target_fps: u32,
    pub within_budget: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PerformanceReport {
    pub schema_version: String,
    pub metadata: RunMetadata,
    pub seed: SeedInfo,
    pub scenario_path: String,
    pub result: ResultEnvelope,
    pub metrics: PerformanceMetrics,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<ArtifactDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TestSuiteResult {
    pub name: String,
    pub passed: u32,
    pub failed: u32,
    pub ignored: u32,
    pub duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_artifact: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TestResultBundle {
    pub schema_version: String,
    pub metadata: RunMetadata,
    pub seed: SeedInfo,
    pub result: ResultEnvelope,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suites: Vec<TestSuiteResult>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<ArtifactDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<String>>,
}

pub fn canonical_noop_test_result_bundle(
    metadata: RunMetadata,
    seed: SeedInfo,
    terminal_report_path: impl Into<String>,
) -> TestResultBundle {
    let terminal_report_path = terminal_report_path.into();

    TestResultBundle {
        schema_version: HARNESS_SCHEMA_VERSION.to_owned(),
        metadata,
        seed,
        result: ResultEnvelope {
            status: HarnessStatus::Passed,
            summary: "No-op harness bundle emitted successfully.".to_owned(),
            failure_kind: None,
            details: Some(
                "Bootstrap contract bundle only; scenario execution and capture work land in later roadmap tasks."
                    .to_owned(),
            ),
        },
        suites: vec![TestSuiteResult {
            name: "noop_harness_contract".to_owned(),
            passed: 1,
            failed: 0,
            ignored: 0,
            duration_ms: 0,
            stdout_artifact: None,
            stderr_artifact: None,
        }],
        artifacts: vec![ArtifactDescriptor {
            role: "terminal_report".to_owned(),
            path: terminal_report_path,
            media_type: "application/json".to_owned(),
        }],
        notes: Some(vec![
            "Artifact paths are stable under reports/harness/<command>/<run_id>/.".to_owned(),
            "This bundle is the bootstrap reference for the agent-facing harness contract.".to_owned(),
        ]),
    }
}

pub fn init_schema_catalog_json() -> BTreeMap<String, serde_json::Value> {
    let schemas = [
        (
            "scenario_request",
            serde_json::to_value(schema_for!(ScenarioRequest))
                .expect("scenario request schema should serialize"),
        ),
        (
            "capture_request",
            serde_json::to_value(schema_for!(CaptureRequest))
                .expect("capture request schema should serialize"),
        ),
        (
            "lookdev_sweep_request",
            serde_json::to_value(schema_for!(LookdevSweepRequest))
                .expect("lookdev request schema should serialize"),
        ),
        (
            "duel_report",
            serde_json::to_value(schema_for!(DuelReport))
                .expect("duel report schema should serialize"),
        ),
        (
            "performance_report",
            serde_json::to_value(schema_for!(PerformanceReport))
                .expect("performance report schema should serialize"),
        ),
        (
            "test_result_bundle",
            serde_json::to_value(schema_for!(TestResultBundle))
                .expect("test result bundle schema should serialize"),
        ),
    ];

    schemas.into_iter().map(|(name, schema)| (name.to_owned(), schema)).collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use jsonschema::validator_for;
    use proptest::collection::vec;
    use proptest::option;
    use proptest::prelude::*;
    use wr_telemetry::{PlatformMetadata, RunMetadata, RunTimestamps};

    use super::*;

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../baselines")
            .canonicalize()
            .expect("workspace baselines dir should exist")
    }

    fn canonical_metadata() -> RunMetadata {
        RunMetadata {
            command_name: "noop-harness-report".to_owned(),
            run_id: "golden-fixture".to_owned(),
            git_sha: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            cwd: "/Users/ryanwible/projects/wrela_rust".to_owned(),
            platform: PlatformMetadata {
                os: "macos".to_owned(),
                family: "unix".to_owned(),
                arch: "aarch64".to_owned(),
            },
            timestamps: RunTimestamps {
                started_at_unix_ms: 1_710_000_000_000,
                completed_at_unix_ms: 1_710_000_000_123,
            },
            notes: Some(vec!["Golden bundle fixture for PR-001.".to_owned()]),
        }
    }

    fn canonical_seed() -> SeedInfo {
        SeedInfo {
            label: "hero_forest".to_owned(),
            value_hex: "0xDEADBEEF".to_owned(),
            stream: Some("bootstrap".to_owned()),
        }
    }

    fn canonical_scenario_request() -> ScenarioRequest {
        ScenarioRequest {
            schema_version: HARNESS_SCHEMA_VERSION.to_owned(),
            scenario_path: "scenarios/smoke/startup.ron".to_owned(),
            simulation_rate_hz: 60,
            fixed_steps: 16,
            seed: canonical_seed(),
            scripted_inputs: vec![ScriptedInput {
                frame: 4,
                action: "move_forward".to_owned(),
                state: "pressed".to_owned(),
            }],
            assertions: vec![ScenarioAssertion {
                metric: "startup.frame_count".to_owned(),
                comparator: "eq".to_owned(),
                expected: 16.0,
                tolerance: Some(0.0),
            }],
        }
    }

    fn canonical_capture_request() -> CaptureRequest {
        CaptureRequest {
            schema_version: HARNESS_SCHEMA_VERSION.to_owned(),
            scenario_path: "scenarios/traversal/hero_path.ron".to_owned(),
            camera_set: "forest_hero".to_owned(),
            frame_count: 8,
            seed: canonical_seed(),
            requested_outputs: vec!["png".to_owned(), "metrics.json".to_owned()],
        }
    }

    fn canonical_lookdev_sweep_request() -> LookdevSweepRequest {
        LookdevSweepRequest {
            schema_version: HARNESS_SCHEMA_VERSION.to_owned(),
            tweak_pack_path: "tweak_packs/release/hero_forest.ron".to_owned(),
            camera_set: "forest_hero".to_owned(),
            seed: canonical_seed(),
            variants: vec![LookdevVariant {
                variant_id: "hero_grade_push".to_owned(),
                overrides: BTreeMap::from([
                    ("atmosphere.mie_strength".to_owned(), "1.15".to_owned()),
                    ("post.grade.warmth".to_owned(), "0.20".to_owned()),
                ]),
            }],
        }
    }

    fn canonical_duel_report() -> DuelReport {
        DuelReport {
            schema_version: HARNESS_SCHEMA_VERSION.to_owned(),
            metadata: canonical_metadata(),
            seed: canonical_seed(),
            scenario_path: "scenarios/duel/wraith_smoke.ron".to_owned(),
            result: ResultEnvelope {
                status: HarnessStatus::Passed,
                summary: "Canonical duel smoke scenario stayed within guardrails.".to_owned(),
                failure_kind: None,
                details: None,
            },
            metrics: DuelMetrics {
                duration_ms: 12_500,
                clash_count: 3,
                player_hits: 2,
                enemy_hits: 1,
                average_reengage_time_ms: 410.0,
            },
            artifacts: vec![ArtifactDescriptor {
                role: "duel_report".to_owned(),
                path: "reports/harness/run-scenario/wraith-smoke/duel_report.json".to_owned(),
                media_type: "application/json".to_owned(),
            }],
            notes: Some(vec!["Bootstrap sample only; combat logic lands later.".to_owned()]),
        }
    }

    fn canonical_performance_report() -> PerformanceReport {
        PerformanceReport {
            schema_version: HARNESS_SCHEMA_VERSION.to_owned(),
            metadata: canonical_metadata(),
            seed: canonical_seed(),
            scenario_path: "scenarios/traversal/hero_path.ron".to_owned(),
            result: ResultEnvelope {
                status: HarnessStatus::Passed,
                summary: "Reference perf envelope is within the bootstrap target.".to_owned(),
                failure_kind: None,
                details: None,
            },
            metrics: PerformanceMetrics {
                average_frame_ms: 14.8,
                p95_frame_ms: 16.3,
                target_fps: 60,
                within_budget: true,
            },
            artifacts: vec![ArtifactDescriptor {
                role: "perf_metrics".to_owned(),
                path: "reports/harness/perf/hero-path/perf_report.json".to_owned(),
                media_type: "application/json".to_owned(),
            }],
            notes: None,
        }
    }

    fn canonical_test_result_bundle() -> TestResultBundle {
        canonical_noop_test_result_bundle(
            canonical_metadata(),
            canonical_seed(),
            "reports/harness/noop-harness-report/golden-fixture/terminal_report.json",
        )
    }

    #[test]
    fn contract_examples_roundtrip_through_json() {
        let scenario = canonical_scenario_request();
        let capture = canonical_capture_request();
        let lookdev = canonical_lookdev_sweep_request();
        let duel = canonical_duel_report();
        let perf = canonical_performance_report();
        let bundle = canonical_test_result_bundle();

        let scenario_json = serde_json::to_string(&scenario).expect("scenario serializes");
        let capture_json = serde_json::to_string(&capture).expect("capture serializes");
        let lookdev_json = serde_json::to_string(&lookdev).expect("lookdev serializes");
        let duel_json = serde_json::to_string(&duel).expect("duel serializes");
        let perf_json = serde_json::to_string(&perf).expect("perf serializes");
        let bundle_json = serde_json::to_string(&bundle).expect("bundle serializes");

        assert_eq!(
            serde_json::from_str::<ScenarioRequest>(&scenario_json).expect("scenario roundtrips"),
            scenario
        );
        assert_eq!(
            serde_json::from_str::<CaptureRequest>(&capture_json).expect("capture roundtrips"),
            capture
        );
        assert_eq!(
            serde_json::from_str::<LookdevSweepRequest>(&lookdev_json).expect("lookdev roundtrips"),
            lookdev
        );
        assert_eq!(serde_json::from_str::<DuelReport>(&duel_json).expect("duel roundtrips"), duel);
        assert_eq!(
            serde_json::from_str::<PerformanceReport>(&perf_json).expect("perf roundtrips"),
            perf
        );
        assert_eq!(
            serde_json::from_str::<TestResultBundle>(&bundle_json).expect("bundle roundtrips"),
            bundle
        );
    }

    #[test]
    fn generated_schemas_validate_canonical_examples() {
        let schemas = init_schema_catalog_json();
        let examples = BTreeMap::from([
            (
                "scenario_request".to_owned(),
                serde_json::to_value(canonical_scenario_request()).expect("scenario fixture"),
            ),
            (
                "capture_request".to_owned(),
                serde_json::to_value(canonical_capture_request()).expect("capture fixture"),
            ),
            (
                "lookdev_sweep_request".to_owned(),
                serde_json::to_value(canonical_lookdev_sweep_request()).expect("lookdev fixture"),
            ),
            (
                "duel_report".to_owned(),
                serde_json::to_value(canonical_duel_report()).expect("duel fixture"),
            ),
            (
                "performance_report".to_owned(),
                serde_json::to_value(canonical_performance_report()).expect("perf fixture"),
            ),
            (
                "test_result_bundle".to_owned(),
                serde_json::to_value(canonical_test_result_bundle()).expect("bundle fixture"),
            ),
        ]);

        for (name, schema) in schemas {
            let validator = validator_for(&schema).unwrap_or_else(|error| {
                panic!("schema `{name}` should compile for validation: {error}");
            });
            validator.validate(examples.get(&name).expect("example should exist")).unwrap_or_else(
                |error| panic!("schema `{name}` should accept canonical example: {error}"),
            );
        }
    }

    #[test]
    fn canonical_noop_bundle_matches_checked_in_golden_file() {
        let expected =
            std::fs::read_to_string(fixture_root().join("reports/noop_test_result_bundle_v1.json"))
                .expect("golden report fixture should exist");

        let actual = serde_json::to_string_pretty(&canonical_test_result_bundle())
            .expect("canonical bundle should serialize");

        assert_eq!(actual, expected.trim_end());
    }

    proptest! {
        #[test]
        fn optional_fields_remain_backward_compatible_when_omitted(
            metadata_notes in option::of(vec("[a-z]{1,12}", 0..3)),
            metadata_stream in option::of("[a-z]{1,12}"),
            bundle_notes in option::of(vec("[a-z]{1,12}", 0..3)),
            detail in option::of("[a-z]{1,24}"),
        ) {
            let mut bundle = canonical_test_result_bundle();
            bundle.metadata.notes = metadata_notes.clone();
            bundle.seed.stream = metadata_stream.clone();
            bundle.notes = bundle_notes.clone();
            bundle.result.details = detail.clone();

            let json = serde_json::to_string(&bundle).expect("bundle serializes");
            let reparsed: TestResultBundle = serde_json::from_str(&json).expect("bundle reparses");

            prop_assert_eq!(reparsed, bundle);
        }
    }
}
