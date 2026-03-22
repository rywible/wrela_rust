use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{HarnessError, TestResultBundle};

pub const HARNESS_REPORTS_ROOT: &str = "reports/harness";
pub const TERMINAL_REPORT_FILENAME: &str = "terminal_report.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactLayout {
    command_name: String,
    run_id: String,
}

impl ArtifactLayout {
    pub fn new(command_name: impl Into<String>, run_id: impl Into<String>) -> Self {
        Self { command_name: command_name.into(), run_id: run_id.into() }
    }

    pub fn run_directory(&self) -> PathBuf {
        Path::new(HARNESS_REPORTS_ROOT).join(&self.command_name).join(&self.run_id)
    }

    pub fn terminal_report_path(&self) -> PathBuf {
        self.run_directory().join(TERMINAL_REPORT_FILENAME)
    }

    pub fn terminal_report_path_string(&self) -> String {
        self.terminal_report_path().to_string_lossy().into_owned()
    }
}

pub fn write_test_result_bundle(
    layout: &ArtifactLayout,
    bundle: &TestResultBundle,
) -> Result<PathBuf, HarnessError> {
    write_test_result_bundle_at(Path::new("."), layout, bundle)
}

pub fn write_test_result_bundle_at(
    root: &Path,
    layout: &ArtifactLayout,
    bundle: &TestResultBundle,
) -> Result<PathBuf, HarnessError> {
    let path = root.join(layout.terminal_report_path());
    let parent = path
        .parent()
        .ok_or_else(|| HarnessError::invalid_path(path.to_string_lossy().into_owned()))?;

    fs::create_dir_all(parent)?;
    let mut json = serde_json::to_vec_pretty(bundle)?;
    json.push(b'\n');
    fs::write(&path, json)?;
    Ok(path)
}

pub fn write_json_artifact<T: Serialize>(
    layout: &ArtifactLayout,
    filename: &str,
    artifact: &T,
) -> Result<PathBuf, HarnessError> {
    write_json_artifact_at(Path::new("."), layout, filename, artifact)
}

pub fn write_json_artifact_at<T: Serialize>(
    root: &Path,
    layout: &ArtifactLayout,
    filename: &str,
    artifact: &T,
) -> Result<PathBuf, HarnessError> {
    let path = root.join(layout.run_directory()).join(filename);
    let parent = path
        .parent()
        .ok_or_else(|| HarnessError::invalid_path(path.to_string_lossy().into_owned()))?;

    fs::create_dir_all(parent)?;
    let mut json = serde_json::to_vec_pretty(artifact)?;
    json.push(b'\n');
    fs::write(&path, json)?;
    Ok(path)
}
