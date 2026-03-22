use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStepStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VerificationStepRecord {
    pub suite_name: String,
    pub suite_slug: String,
    pub command: Vec<String>,
    pub status: VerificationStepStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_artifact: Option<String>,
}

impl VerificationStepRecord {
    pub fn new(
        suite_name: impl Into<String>,
        command: Vec<String>,
        status: VerificationStepStatus,
        duration_ms: u64,
    ) -> Self {
        let suite_name = suite_name.into();

        Self {
            suite_slug: artifact_component(&suite_name),
            suite_name,
            command,
            status,
            exit_code: None,
            duration_ms,
            stdout_artifact: None,
            stderr_artifact: None,
        }
    }
}

pub fn artifact_component(label: &str) -> String {
    let mut slug = String::with_capacity(label.len());
    let mut saw_component = false;
    let mut pending_dash = false;

    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && saw_component {
                slug.push('-');
            }
            slug.push(ch.to_ascii_lowercase());
            saw_component = true;
            pending_dash = false;
        } else if saw_component {
            pending_dash = true;
        }
    }

    if slug.is_empty() { String::from("unnamed") } else { slug }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use insta::assert_json_snapshot;
    use proptest::prelude::*;
    use proptest::test_runner::{
        Config, FileFailurePersistence, TestCaseError, TestError, TestRunner,
    };
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn canonical_verification_step_records_match_snapshot() {
        let mut fmt = VerificationStepRecord::new(
            "fmt / workspace",
            vec!["cargo".to_owned(), "fmt".to_owned(), "--check".to_owned()],
            VerificationStepStatus::Passed,
            24,
        );
        fmt.stdout_artifact =
            Some("reports/harness/verify/demo/fmt-workspace.stdout.log".to_owned());

        let mut nextest = VerificationStepRecord::new(
            "nextest::workspace",
            vec![
                "cargo".to_owned(),
                "nextest".to_owned(),
                "run".to_owned(),
                "--workspace".to_owned(),
                "--profile".to_owned(),
                "ci".to_owned(),
            ],
            VerificationStepStatus::Passed,
            231,
        );
        nextest.exit_code = Some(0);
        nextest.stdout_artifact =
            Some("reports/harness/verify/demo/nextest-workspace.events.jsonl".to_owned());
        nextest.stderr_artifact =
            Some("reports/harness/verify/demo/nextest-workspace.stderr.log".to_owned());

        assert_json_snapshot!("verification_step_records", vec![fmt, nextest]);
    }

    #[test]
    fn failing_property_test_shrinks_and_persists_minimal_repro() {
        let temp = tempdir().expect("temporary directory should be created");
        let failure_path = temp.path().join("telemetry-regressions.txt");
        let failure_path: &'static str =
            Box::leak(failure_path.to_string_lossy().into_owned().into_boxed_str());

        let config = Config {
            failure_persistence: Some(Box::new(FileFailurePersistence::Direct(failure_path))),
            cases: 16,
            ..Config::default()
        };

        let failure = TestRunner::new(config)
            .run(&(0u8..=255), |value| {
                if value < 3 {
                    Ok(())
                } else {
                    Err(TestCaseError::fail(format!("value {value} should be below 3")))
                }
            })
            .expect_err("the property runner should find a failing input");

        match failure {
            TestError::Fail(_, minimal) => assert_eq!(minimal, 3),
            other => panic!("expected a minimal failing input, got {other:?}"),
        }

        let persisted = fs::read_to_string(failure_path)
            .expect("the failure persistence file should be written");
        assert!(
            persisted.contains("3"),
            "the persisted repro should mention the shrunk minimal input: {persisted}"
        );
    }

    proptest! {
        #[test]
        fn artifact_component_produces_ascii_path_safe_slugs(
            label in "[A-Za-z0-9 _./:-]{0,64}"
        ) {
            let slug = artifact_component(&label);

            prop_assert!(!slug.is_empty());
            prop_assert!(slug.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'));
            prop_assert!(!slug.starts_with('-'));
            prop_assert!(!slug.ends_with('-'));
            prop_assert!(!slug.contains("--"));

            let input_has_alnum = label.chars().any(|ch| ch.is_ascii_alphanumeric());
            if input_has_alnum {
                prop_assert_ne!(slug, "unnamed");
            } else {
                prop_assert_eq!(slug, "unnamed");
            }
        }
    }
}
