#![forbid(unsafe_code)]

mod artifact_paths;
mod contract;

pub use artifact_paths::{
    ArtifactLayout, HARNESS_REPORTS_ROOT, TERMINAL_REPORT_FILENAME, write_json_artifact,
    write_json_artifact_at, write_test_result_bundle, write_test_result_bundle_at,
};
pub use contract::{
    ArtifactDescriptor, CaptureRequest, CommandExecutionReport, DaemonCommandRequest,
    DaemonJobSnapshot, DaemonJobState, DaemonLaunchRequest, DuelMetrics, DuelReport, FailureKind,
    HARNESS_SCHEMA_VERSION, HarnessError, HarnessStatus, LookdevSweepRequest, LookdevVariant,
    PerformanceMetrics, PerformanceReport, ResultEnvelope, SUPPORTED_ASSERTION_COMPARATORS,
    ScenarioActorSpawn, ScenarioAssertion, ScenarioAssertionResult, ScenarioExecutionMetrics,
    ScenarioExecutionReport, ScenarioRequest, ScriptedInput, TestResultBundle, TestSuiteResult,
    canonical_noop_test_result_bundle, init_schema_catalog_json, load_scenario_request_ron,
};

use wr_core::{CrateBoundary, CrateEntryPoint};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_tools_harness", CrateBoundary::Subsystem, false)
}
