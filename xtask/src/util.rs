use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn current_git_sha() -> Result<String, String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|error| format!("failed to invoke git: {error}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }

    let sha = String::from_utf8(output.stdout)
        .map_err(|error| format!("git returned non-utf8 output: {error}"))?;
    let trimmed = sha.trim();

    if trimmed.is_empty() {
        return Err(String::from("git returned an empty sha"));
    }

    Ok(trimmed.to_owned())
}

pub(crate) fn now_unix_ms() -> Result<u64, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock is before unix epoch: {error}"))?;

    u64::try_from(duration.as_millis())
        .map_err(|_| String::from("unix millisecond timestamp overflowed u64"))
}
