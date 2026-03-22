#![forbid(unsafe_code)]

mod artifact_paths;
mod contract;

pub use artifact_paths::{
    ArtifactLayout, HARNESS_REPORTS_ROOT, TERMINAL_REPORT_FILENAME, write_test_result_bundle,
    write_test_result_bundle_at,
};
pub use contract::{
    ArtifactDescriptor, CaptureRequest, DuelMetrics, DuelReport, FailureKind, HarnessError,
    HarnessStatus, LookdevSweepRequest, LookdevVariant, PerformanceMetrics, PerformanceReport,
    ResultEnvelope, ScenarioAssertion, ScenarioRequest, ScriptedInput, TestResultBundle,
    TestSuiteResult, canonical_noop_test_result_bundle, init_schema_catalog_json,
};

use wr_core::{CrateBoundary, CrateEntryPoint};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_tools_harness", CrateBoundary::Subsystem, false)
}
