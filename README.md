# wrela_rust

Wrela v0 is an autonomous-first Rust game runtime project for a narrow procedural vertical slice: a seed-driven redwood forest, fixed late-afternoon lighting, a floating telekinetic katana player embodiment, one wraith archetype, and a deterministic harness-first workflow.

## Bootstrap Status

This repository is in bootstrap phase right now.

- The process, roadmap, issue forms, PR packet, and task tracking system exist.
- The Rust workspace, crates, apps, `xtask`, scenarios, and harness command surface are planned but not all landed yet.
- The default next implementation task is [`PR-000`](roadmap/pending_tasks/PR-000-workspace-scaffold-conventions-and-empty-crate-topology.md).

## Current Source Of Truth

Use these files first:

- [`AGENTS.md`](AGENTS.md) for the top-level operating contract.
- [`roadmap/wrela_v0_pr_backlog.json`](roadmap/wrela_v0_pr_backlog.json) for machine-readable task scope and dependency order.
- [`roadmap/wrela_v0_rust_project_plan.md`](roadmap/wrela_v0_rust_project_plan.md) for the human-readable roadmap narrative.
- [`roadmap/pending_tasks/`](roadmap/pending_tasks/) and [`roadmap/completed_tasks/`](roadmap/completed_tasks/) for execution status.
- [`docs/process/TRIAGE_GUIDELINES.md`](docs/process/TRIAGE_GUIDELINES.md) for routing and readiness rules.
- [`docs/process/CODE_REVIEW_GUIDELINES.md`](docs/process/CODE_REVIEW_GUIDELINES.md) for review and merge discipline.
- [`.github/ISSUE_TEMPLATE/`](.github/ISSUE_TEMPLATE/) for structured issue intake.
- [`.github/PULL_REQUEST_TEMPLATE.md`](.github/PULL_REQUEST_TEMPLATE.md) for the PR evidence contract.
- [`docs/process/process_contract.json`](docs/process/process_contract.json) and [`docs/process/validate_process_contract.py`](docs/process/validate_process_contract.py) for bootstrap process validation.

## Working In This Repo Today

If you are operating autonomously right now:

1. Start from a named task.
2. Prefer the earliest unblocked dependency task.
3. Put implementation work on its own task branch.
4. Open a GitHub PR for the task before merge.
5. Push when the work is ready, wait for automated review, and read the result.
6. If review feedback arrives, fix it, push again, and wait for the next review cycle.
7. Merge only after the latest push gets an explicit green light such as `thumbs up`, unless a human waives that requirement.
8. Move a completed roadmap task file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in the same PR that lands the work.
9. Do not claim Cargo or harness verification that does not exist yet.

## Bootstrap Verification

Run the process validator before and after process or roadmap changes:

```bash
python3 docs/process/validate_process_contract.py
```

That validator checks the bootstrap operating contract, including:

- required process files,
- the GitHub issue form directory path,
- backlog JSON readability,
- one-to-one roadmap task coverage across pending and completed task folders,
- duplicate or stray task IDs.

## Target End-State Automation Surface

These commands are the required product surface after the scaffold and harness tasks land. They are not all available yet.

```bash
cargo xtask verify
cargo xtask run-scenario scenarios/smoke/startup.ron
cargo xtask run-scenario scenarios/duel/wraith_smoke.ron
cargo xtask lookdev --seed 0xDEADBEEF --pack tweak_packs/release/hero_forest.ron --camera-set forest_hero
cargo xtask capture --scenario scenarios/traversal/hero_path.ron
cargo xtask perf --scenario scenarios/traversal/perf_path.ron
cargo xtask replay baselines/replays/wraith_duel_seed01.json
cargo xtask daemon
```

Until those exist, missing commands are backlog work to implement, not permission to create private replacement contracts.
