#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import subprocess
import sys
import time
from typing import Any


SCHEMA_VERSION = "claude_review_harness/v2"
# v2 keeps stdout.txt as the extracted review text for backward compatibility and
# adds raw_response.json for the raw Claude JSON envelope plus richer metadata.


def repo_root() -> pathlib.Path:
    return pathlib.Path(__file__).resolve().parents[1]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run the local Claude Code review loop with a stable prompt, timeout, and artifact path."
    )
    parser.add_argument("--base-ref", default="origin/main", help="Git ref to review against.")
    parser.add_argument(
        "--timeout-seconds",
        type=int,
        default=600,
        help="Maximum wall-clock time to wait for Claude before classifying the run as timed out.",
    )
    parser.add_argument(
        "--run-id",
        default=None,
        help="Optional run id. Defaults to claude-review-<unix-ms>.",
    )
    parser.add_argument(
        "--reports-root",
        default="reports/process/claude-review",
        help="Repo-relative directory that will receive the review artifacts.",
    )
    parser.add_argument(
        "--policy-path",
        default="docs/process/CODE_REVIEW_GUIDELINES.md",
        help="Repo-relative path to the review policy document.",
    )
    parser.add_argument(
        "--claude-binary",
        default="claude",
        help="Binary name or path for the Claude CLI.",
    )

    args = parser.parse_args()
    if args.timeout_seconds <= 0:
        parser.error("--timeout-seconds must be greater than zero")
    return args


def current_git_value(root: pathlib.Path, *args: str, fallback: str) -> str:
    try:
        completed = subprocess.run(
            ["git", *args],
            cwd=root,
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return fallback

    value = completed.stdout.strip()
    return value if value else fallback


def build_prompt(policy_path: str, base_ref: str) -> str:
    return "\n".join(
        [
            f"Use {policy_path} as the review policy for this repository.",
            f"Review the current git branch against {base_ref}.",
            "Focus on bugs, behavioral regressions, determinism risks, missing tests, schema or contract gaps, and process violations.",
            "Return findings first, ordered by severity, with file references where possible.",
            "If there are no findings, say that explicitly and mention residual risks.",
        ]
    )


def relpath(path: pathlib.Path, root: pathlib.Path) -> str:
    return str(path.relative_to(root))


def write_text(path: pathlib.Path, text: str) -> None:
    path.write_text(text, encoding="utf-8")


def write_json(path: pathlib.Path, payload: object) -> None:
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def primary_model_name(payload: dict[str, Any]) -> str | None:
    model_usage = payload.get("modelUsage")
    if not isinstance(model_usage, dict) or not model_usage:
        return None
    # Claude currently reports model usage as a single-entry object for this flow.
    model_name = next(iter(model_usage.keys()))
    return model_name if isinstance(model_name, str) else None


def classify_claude_stdout(stdout: str) -> tuple[str, str, dict[str, Any] | None, str | None]:
    stripped = stdout.strip()
    if not stripped:
        return ("completed_empty_output", "", None, None)

    try:
        payload = json.loads(stripped)
    except json.JSONDecodeError as exc:
        location = f"line {exc.lineno}, column {exc.colno}"
        return ("completed_invalid_output", "", None, f"{exc.msg} ({location})")

    if not isinstance(payload, dict):
        return (
            "completed_invalid_output",
            "",
            None,
            f"Expected Claude JSON output to be an object, got {type(payload).__name__}.",
        )

    review_text = payload.get("result")
    if review_text is None:
        return ("completed_empty_output", "", payload, None)
    if not isinstance(review_text, str):
        return (
            "completed_invalid_output",
            "",
            payload,
            f"Expected Claude JSON output 'result' to be a string, got {type(review_text).__name__}.",
        )
    if not review_text.strip():
        return ("completed_empty_output", "", payload, None)

    return ("completed", review_text.rstrip() + "\n", payload, None)


def exit_code_for_status(status: str) -> int:
    return {
        "completed": 0,
        "completed_empty_output": 3,
        "completed_invalid_output": 6,
        "exit_nonzero": 4,
        "timeout": 5,
    }.get(status, 1)


def run_claude_attempt(command: list[str], cwd: pathlib.Path, timeout_seconds: int) -> dict[str, Any]:
    attempt_started = int(time.time() * 1000)
    exit_code: int | None = None
    timed_out = False
    raw_response = ""
    stderr = ""
    review_text = ""
    claude_payload: dict[str, Any] | None = None
    parse_error: str | None = None
    status = "completed"

    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
        )
        exit_code = completed.returncode
        raw_response = completed.stdout or ""
        stderr = completed.stderr or ""
        if exit_code != 0:
            status = "exit_nonzero"
        else:
            status, review_text, claude_payload, parse_error = classify_claude_stdout(raw_response)
    except subprocess.TimeoutExpired as exc:
        timed_out = True
        status = "timeout"
        raw_response = exc.stdout or ""
        stderr = exc.stderr or ""
        _, review_text, claude_payload, parse_error = classify_claude_stdout(raw_response)

    attempt_completed = int(time.time() * 1000)
    summary = {
        "status": status,
        "exit_code": exit_code,
        "timed_out": timed_out,
        "timeout_seconds": timeout_seconds,
        "started_at_unix_ms": attempt_started,
        "completed_at_unix_ms": attempt_completed,
        "stdout_bytes": len(review_text.encode("utf-8")),
        "raw_response_bytes": len(raw_response.encode("utf-8")),
        "stderr_bytes": len(stderr.encode("utf-8")),
    }
    if parse_error is not None:
        summary["parse_error"] = parse_error
    if claude_payload is not None:
        summary["claude_duration_ms"] = claude_payload.get("duration_ms")
        summary["claude_response_type"] = claude_payload.get("type")
        summary["claude_response_subtype"] = claude_payload.get("subtype")
        summary["claude_session_id"] = claude_payload.get("session_id")
        summary["claude_stop_reason"] = claude_payload.get("stop_reason")
        summary["claude_model"] = primary_model_name(claude_payload)

    return {
        "summary": summary,
        "status": status,
        "exit_code": exit_code,
        "timed_out": timed_out,
        "raw_response": raw_response,
        "stderr": stderr,
        "review_text": review_text,
        "claude_payload": claude_payload,
        "parse_error": parse_error,
    }


def main() -> int:
    args = parse_args()
    root = repo_root()
    run_id = args.run_id or f"claude-review-{int(time.time() * 1000)}"
    reports_root = root / args.reports_root
    run_dir = reports_root / run_id
    run_dir.mkdir(parents=True, exist_ok=True)

    prompt = build_prompt(args.policy_path, args.base_ref)
    prompt_path = run_dir / "prompt.txt"
    raw_response_path = run_dir / "raw_response.json"
    stdout_path = run_dir / "stdout.txt"
    stderr_path = run_dir / "stderr.txt"
    result_path = run_dir / "result.json"

    write_text(prompt_path, prompt + "\n")

    command = [
        args.claude_binary,
        "--permission-mode",
        "bypassPermissions",
        "--dangerously-skip-permissions",
        "--output-format",
        "json",
        "-p",
        prompt,
    ]

    started_at = int(time.time() * 1000)
    started_monotonic = time.monotonic()
    attempts: list[dict[str, Any]] = []
    attempt_result: dict[str, Any] | None = None

    for attempt_number in range(1, 3):
        elapsed_seconds = time.monotonic() - started_monotonic
        remaining_seconds = args.timeout_seconds - elapsed_seconds
        if remaining_seconds <= 0:
            attempt_result = {
                "summary": {
                    "status": "timeout",
                    "exit_code": None,
                    "timed_out": True,
                    "timeout_seconds": 0,
                    "started_at_unix_ms": int(time.time() * 1000),
                    "completed_at_unix_ms": int(time.time() * 1000),
                    "stdout_bytes": 0,
                    "raw_response_bytes": 0,
                    "stderr_bytes": 0,
                },
                "status": "timeout",
                "exit_code": None,
                "timed_out": True,
                "raw_response": "",
                "stderr": "",
                "review_text": "",
                "claude_payload": None,
                "parse_error": None,
            }
            attempts.append({"attempt": attempt_number, **attempt_result["summary"]})
            break

        timeout_seconds = max(1, int(remaining_seconds))
        attempt_result = run_claude_attempt(command, root, timeout_seconds)
        attempts.append({"attempt": attempt_number, **attempt_result["summary"]})
        if attempt_result["status"] not in {"completed_empty_output", "completed_invalid_output"}:
            break

    if attempt_result is None:
        raise RuntimeError("Claude review wrapper did not execute any attempts.")

    status = attempt_result["status"]
    exit_code = attempt_result["exit_code"]
    timed_out = attempt_result["timed_out"]
    raw_response = attempt_result["raw_response"]
    stderr = attempt_result["stderr"]
    review_text = attempt_result["review_text"]
    claude_payload = attempt_result["claude_payload"]
    parse_error = attempt_result["parse_error"]

    completed_at = int(time.time() * 1000)
    write_text(raw_response_path, raw_response)
    write_text(stdout_path, review_text)
    write_text(stderr_path, stderr)

    payload = {
        "schema_version": SCHEMA_VERSION,
        "status": status,
        "timeout_seconds": args.timeout_seconds,
        "timed_out": timed_out,
        "exit_code": exit_code,
        "run_id": run_id,
        "base_ref": args.base_ref,
        "policy_path": args.policy_path,
        "cwd": str(root),
        "git_sha": current_git_value(root, "rev-parse", "HEAD", fallback="<unknown>"),
        "branch": current_git_value(root, "rev-parse", "--abbrev-ref", "HEAD", fallback="<unknown>"),
        "started_at_unix_ms": started_at,
        "completed_at_unix_ms": completed_at,
        "prompt_sha256": hashlib.sha256(prompt.encode("utf-8")).hexdigest(),
        "stdout_artifact": relpath(stdout_path, root),
        "raw_response_artifact": relpath(raw_response_path, root),
        "stderr_artifact": relpath(stderr_path, root),
        "prompt_artifact": relpath(prompt_path, root),
        "stdout_bytes": len(review_text.encode("utf-8")),
        "raw_response_bytes": len(raw_response.encode("utf-8")),
        "stderr_bytes": len(stderr.encode("utf-8")),
        "stdout_format": "review_text",
        "raw_response_format": "claude_json",
        "attempt_count": len(attempts),
        "attempts": attempts,
        "command": [
            args.claude_binary,
            "--permission-mode",
            "bypassPermissions",
            "--dangerously-skip-permissions",
            "--output-format",
            "json",
            "-p",
            "<prompt written to prompt.txt>",
        ],
    }
    if parse_error is not None:
        payload["parse_error"] = parse_error
    if claude_payload is not None:
        payload["claude_duration_ms"] = claude_payload.get("duration_ms")
        payload["claude_response_type"] = claude_payload.get("type")
        payload["claude_response_subtype"] = claude_payload.get("subtype")
        payload["claude_session_id"] = claude_payload.get("session_id")
        payload["claude_stop_reason"] = claude_payload.get("stop_reason")
        payload["claude_model"] = primary_model_name(claude_payload)
    write_json(result_path, payload)

    print(relpath(result_path, root))
    return exit_code_for_status(status)


if __name__ == "__main__":
    sys.exit(main())
