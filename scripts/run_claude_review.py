#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import subprocess
import sys
import time


SCHEMA_VERSION = "claude_review_harness/v1"


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


def exit_code_for_status(status: str) -> int:
    return {
        "completed": 0,
        "completed_empty_output": 3,
        "exit_nonzero": 4,
        "timeout": 5,
    }.get(status, 1)


def main() -> int:
    args = parse_args()
    root = repo_root()
    run_id = args.run_id or f"claude-review-{int(time.time() * 1000)}"
    reports_root = root / args.reports_root
    run_dir = reports_root / run_id
    run_dir.mkdir(parents=True, exist_ok=True)

    prompt = build_prompt(args.policy_path, args.base_ref)
    prompt_path = run_dir / "prompt.txt"
    stdout_path = run_dir / "stdout.txt"
    stderr_path = run_dir / "stderr.txt"
    result_path = run_dir / "result.json"

    write_text(prompt_path, prompt + "\n")

    command = [
        args.claude_binary,
        "--permission-mode",
        "bypassPermissions",
        "--dangerously-skip-permissions",
        "-p",
        prompt,
    ]

    started_at = int(time.time() * 1000)
    status = "completed"
    exit_code: int | None = None
    timed_out = False
    stdout = ""
    stderr = ""

    try:
        completed = subprocess.run(
            command,
            cwd=root,
            capture_output=True,
            text=True,
            timeout=args.timeout_seconds,
        )
        exit_code = completed.returncode
        stdout = completed.stdout or ""
        stderr = completed.stderr or ""
        if exit_code != 0:
            status = "exit_nonzero"
        elif not stdout.strip():
            status = "completed_empty_output"
    except subprocess.TimeoutExpired as exc:
        timed_out = True
        status = "timeout"
        stdout = exc.stdout or ""
        stderr = exc.stderr or ""

    completed_at = int(time.time() * 1000)
    write_text(stdout_path, stdout)
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
        "stderr_artifact": relpath(stderr_path, root),
        "prompt_artifact": relpath(prompt_path, root),
        "stdout_bytes": len(stdout.encode("utf-8")),
        "stderr_bytes": len(stderr.encode("utf-8")),
        "command": [
            args.claude_binary,
            "--permission-mode",
            "bypassPermissions",
            "--dangerously-skip-permissions",
            "-p",
            "<prompt written to prompt.txt>",
        ],
    }
    write_json(result_path, payload)

    print(relpath(result_path, root))
    return exit_code_for_status(status)


if __name__ == "__main__":
    sys.exit(main())
