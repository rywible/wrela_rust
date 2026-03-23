# Local Agent Daemon

`PR-004` introduces the first local-only HTTP wrapper around the harness command surface: `wr_agentd` and `cargo xtask daemon`.

## Purpose

The daemon exists so autonomous tools can launch verification and harness commands through a stable JSON API instead of relying on shell heuristics.

The daemon does not replace the CLI. It wraps the repo-standard CLI entrypoints and preserves their terminal reports as the source of truth.

## API shape

The bootstrap daemon exposes:

- `GET /healthz`
- `POST /v1/jobs`
- `GET /v1/jobs/{job_id}`

`POST /v1/jobs` accepts a `DaemonLaunchRequest`. `GET /v1/jobs/{job_id}` returns a `DaemonJobSnapshot`.

The current public command requests are:

- `verify`
- `run_scenario`
- `capture_frames`
- `lookdev_sweep`
- `perf_check`

## Execution model

Each accepted job:

1. receives a stable daemon job ID,
2. receives or is assigned a stable `run_id` for the underlying CLI command,
3. spawns the matching `cargo xtask ...` subprocess,
4. streams stdout and stderr into daemon-side artifacts,
5. exposes the underlying terminal report path immediately so clients can locate the final machine-readable result without scraping subprocess text.

The daemon-side artifacts live under:

`reports/harness/daemon/<job_id>/`

The wrapped command still writes its own terminal report under:

`reports/harness/<command>/<run_id>/terminal_report.json`

## Bootstrap limits

Only `verify` and `run-scenario` are real command implementations today.

`capture`, `lookdev`, and `perf` are reserved bootstrap wrappers that emit a structured `CommandExecutionReport` explaining that the stable command surface exists before the full runtime work lands.

That keeps the daemon contract stable now while later roadmap tasks replace the placeholder reports with real capture, lookdev, and perf execution.
