# AGENTS.md

This file governs autonomous and human work in this repository. Treat every **MUST** as binding. Treat every **MUST NOT** as a hard stop. If this file conflicts with convenience, convenience loses.

This repository is an autonomous-first build experiment. That changes the rules. The codebase must be written so an agent can build it, test it, tune it, replay it, capture it, and explain its changes without guessing.

## Fast Path Checklist

If you only remember one part of this file, remember this checklist.

For any implementation task, the default end-to-end flow is:

1. Start from a named task and choose the earliest unblocked dependency-correct item.
2. Create or switch to a dedicated task branch before making implementation changes.
3. Land the smallest mergeable change inside the narrowest crate boundary.
4. Add or update tests, reports, baselines, and docs that make the change machine-verifiable.
5. Run the strongest verification that actually exists for the current repo state.
6. Push the branch and open or update the GitHub PR with the required PR packet.
7. If the task is complete, move its task file from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
8. Run local Claude Code review against the current branch, address actionable feedback, rerun verification, and push again if needed.
9. Do not treat local code plus local green checks as task completion unless a human explicitly asked for local-only work.

Local implementation is progress. A task is normally complete only when the GitHub PR exists and the local Claude review loop has been completed or explicitly waived by a human.

## 0. Current Repository Reality

This file describes both the **target runtime** and the **current bootstrap repo**. Agents MUST preserve the aspirational end-state while also acting truthfully about what exists today.

### 0.1 What exists today

At the current bootstrap stage, the repo already contains:

- `AGENTS.md`
- `README.md`
- `roadmap/wrela_v0_rust_project_plan.md`
- `roadmap/wrela_v0_pr_backlog.json`
- `roadmap/pending_tasks/`
- `roadmap/completed_tasks/`
- `docs/process/TRIAGE_GUIDELINES.md`
- `docs/process/CODE_REVIEW_GUIDELINES.md`
- `docs/process/process_contract.json`
- `docs/process/validate_process_contract.py`
- `.github/ISSUE_TEMPLATE/*.yml`
- `.github/PULL_REQUEST_TEMPLATE.md`

The repo does **not** yet necessarily contain the Rust workspace, crates, apps, scenarios, tweak packs, baselines, `xtask`, or automation commands described later in this document.

### 0.2 How to interpret missing paths and commands

- The topology, crates, scenarios, and commands described later in this file are the **required target contract** for Wrela v0.
- If a listed crate, app, scenario, or command does not yet exist on disk, treat it as an unimplemented contract item, not as a local environment mistake.
- Do not pretend a missing workspace or command already exists.
- Do not claim Cargo, `xtask`, scenario, capture, replay, or perf verification ran if the owning task has not landed yet.
- When the repo is still in bootstrap mode, prefer the earliest dependency-blocking task that creates the missing contract surface instead of working around the absence ad hoc.

### 0.3 Bootstrap mode

Until `PR-000` lands, agents are operating in bootstrap mode.

In bootstrap mode:

- most valid work will happen in `roadmap/`, `docs/process/`, `.github/`, and the initial workspace scaffold created by the active task,
- the authoritative runtime design remains the roadmap and the later sections of this file,
- the authoritative execution status is the pending/completed task split,
- verification will often be file-, schema-, and contract-level rather than Cargo-level,
- the correct move is usually to land the narrowest task that creates the next missing piece of repo reality.

## 1. Mission

Build **Wrela v0**: a bespoke Rust game runtime for a single stylized procedural vertical slice.

The shipped slice is:

- a seed-driven static redwood forest biome,
- fixed late-afternoon hero lighting,
- a Hillaire-inspired sky and atmosphere stack,
- a first-person floating telekinetic katana as the player embodiment,
- one silhouette-first wraith archetype with cloth/ribbons and telekinetic blade,
- intent-driven procedural combat with no imported animation clips,
- a harness that lets autonomous agents verify gameplay, capture look-dev, and measure performance.

This repo is **not** a general-purpose engine. It may become general later, but v0 is a narrow runtime with clean enough seams to generalize after it proves itself.

## 2. Scope lock

The following are frozen for v0. Do not widen them without a written ADR and explicit human approval.

- One non-streaming seed-driven hero biome cell.
- One fixed hero time-of-day.
- One player weapon.
- One enemy archetype.
- One duel loop: 1v1.
- No imported GLB, FBX, skeletal meshes, animation clips, or prebaked authored world assets.
- No open-world streaming.
- No day/night cycle.
- No ongoing ecological simulation after world generation.
- No destruction, tree cutting, branch breaking, foliage slicing, or terrain deformation.
- No visible player body, hands, or sleeves in v0.
- No large traversal feature set beyond strafe, jump, dash, and duel movement.
- No realism requirement. Stylized AAA is the target.

## 3. End-state acceptance criteria

The project is successful when all of the following are true:

1. The same seed always generates the same hero forest on the same machine.
2. The game launches and runs a playable 1v1 duel in the forest.
3. The player can strafe, jump, dash, light attack, heavy attack, and parry as a floating telekinetic katana.
4. A wraith enemy can read clearly, fight coherently, and look good without authored character assets.
5. Sky, atmosphere, sun, shadows, fog, and post-processing create a strong late-afternoon hero look.
6. The agent harness can build, test, run scenarios, capture images, replay inputs, and emit machine-readable reports.
7. Canonical traversal and duel scenarios hit the current performance budget target: **1080p60 on the Mac M4 development machine**, or fail with quantified reports and explicit cut recommendations.

## 4. Product pillars

These are non-negotiable. Do not violate them for short-term speed.

1. **Procedural first.** Important world, enemy, weapon, and motion content must come from code, math, simulation, or parameterized generators.
2. **Deterministic by construction.** Everything important must be reproducible from seed + tweak pack + input trace.
3. **Headless-verifiable.** Every important system must be testable without driving the live client, unless there is a written reason it cannot be.
4. **Tweakable live.** Look-dev and combat-feel systems must expose meaningful knobs through the tweak registry.
5. **Observability over vibes.** The agent must be able to inspect metrics, reports, captures, and replay traces instead of inferring success from logs.
6. **Style over purity.** This runtime is allowed to cheat aggressively if the result reads better and stays testable.
7. **Narrow brilliance over broad abstraction.** Solve the shipped slice first. Do not generalize for imaginary future users.

## 5. Decision hierarchy

When tradeoffs conflict, prioritize in this order:

1. deterministic correctness,
2. mergeable simplicity,
3. observability and testability,
4. frame-time stability,
5. visual quality and feel,
6. future generality.

If you are about to choose a more abstract design because it feels architecturally elegant, stop and prove it improves one of the top four priorities.

## 6. Architecture truths

These are architectural laws for this repo.

### 6.1 Simulation truth and render truth are separate

Gameplay runs in a fixed-step simulation world. Rendering consumes extracted immutable data. Rendering must not own gameplay state.

### 6.2 The harness is a product

The harness is not support code. It is part of the product. It must expose stable commands, machine-readable reports, and reproducible artifact locations.

### 6.3 CPU-authored generation first

For v0, world generation is CPU-driven. GPU work exists to render the world, not to hide untestable generation logic.

### 6.4 Combat is authored-feeling math, not freeform rigid-body fencing

The intended stack is:

`verb -> move family -> trajectory solver -> selective contact/clash resolution -> recovery/parry state changes -> event-driven VFX`

Do not replace this with full unconstrained sword physics.

### 6.5 The wraith wins on silhouette and motion

The wraith is not a realistic humanoid. Its body should read through silhouette, cloth/ribbon motion, emissive accents, fog wisps, and blade behavior. Avoid the uncanny valley on purpose.

## 7. Repository topology and ownership

Do not create new top-level crates casually. The crate graph is a scope control mechanism.
The tree below is the target end-state repository shape. It is not a claim that every path already exists in the current checkout.

```text
wrela-v0/
├─ xtask/
├─ apps/
│  ├─ wr_client/
│  ├─ wr_headless/
│  └─ wr_agentd/
├─ crates/
│  ├─ wr_core/
│  ├─ wr_math/
│  ├─ wr_world_seed/
│  ├─ wr_ecs/
│  ├─ wr_platform/
│  ├─ wr_render_api/
│  ├─ wr_render_wgpu/
│  ├─ wr_render_atmo/
│  ├─ wr_render_scene/
│  ├─ wr_render_post/
│  ├─ wr_world_gen/
│  ├─ wr_procgeo/
│  ├─ wr_physics/
│  ├─ wr_combat/
│  ├─ wr_ai/
│  ├─ wr_actor_player/
│  ├─ wr_actor_wraith/
│  ├─ wr_vfx/
│  ├─ wr_tools_ui/
│  ├─ wr_tools_harness/
│  ├─ wr_telemetry/
│  └─ wr_game/
├─ scenarios/
├─ tweak_packs/
├─ baselines/
├─ docs/
└─ reports/
```

### 7.1 Ownership rules by crate

- `wr_core`: shared types, IDs, config shapes, common errors, no heavy policy.
- `wr_math`: deterministic math helpers, geometry kernels, noise, interpolation, basis functions.
- `wr_world_seed`: seed graph, RNG streams, deterministic subseeding.
- `wr_ecs`: ECS world setup, schedules, stage ordering, plugin registration conventions.
- `wr_platform`: platform shell, timing abstraction, input plumbing, window hooks.
- `wr_render_api`: extracted render data contracts and graph interfaces, no backend specifics.
- `wr_render_wgpu`: `wgpu` backend, pipeline setup, GPU resource management, frame execution.
- `wr_render_atmo`: sky, atmosphere, aerial perspective, fog integration.
- `wr_render_scene`: scene passes, shadows, materials, foliage rendering, extraction adapters.
- `wr_render_post`: post stack, tonemap, bloom, AO, contact shadow cheats, grading.
- `wr_world_gen`: terrain fields, ecological placement, biome generation pipeline.
- `wr_procgeo`: tree graphs, meshes, bark, foliage clusters, ground dressing geometry.
- `wr_physics`: collision wrappers, character kinematics, hit queries, deterministic physics glue.
- `wr_combat`: verbs, move families, trajectory solving, clash/recovery logic, hit windows.
- `wr_ai`: enemy decision logic, spacing, telegraphs, reactions, duel-state selection.
- `wr_actor_player`: player-facing pawn state and presentation extraction.
- `wr_actor_wraith`: wraith body graph, silhouette presentation, cloth/ribbon data, extraction.
- `wr_vfx`: trails, sparks, telekinesis effects, fog wisps, transient visual events.
- `wr_tools_ui`: tweak registry, dev panels, inspectors, in-engine overlays.
- `wr_tools_harness`: scenario loading, report generation, capture orchestration, artifact writing.
- `wr_telemetry`: structured logs, metrics, spans, perf counters, machine-readable reports.
- `wr_game`: composition crate only. It wires subsystems together. It must not absorb subsystem logic.
- `apps/*`: thin app shells. Keep them boring.
- `xtask`: the source of truth for repo automation commands.

### 7.2 Integration boundaries

Before the integration milestones, avoid touching `wr_game`, `apps/wr_client`, and `apps/wr_headless` unless the current task explicitly requires it. Most work should happen inside subsystem crates plus tests.

### 7.3 Plugin rule

Every subsystem crate MUST expose a clear init or plugin entrypoint. No hidden self-registration. Integration code wires systems together explicitly.

## 8. Task source of truth

Autonomous work MUST start from a named task.

Accepted task sources are:

- the project roadmap PR backlog,
- an issue that references a roadmap item,
- a written follow-up generated by a merged PR,
- a human-requested task with explicit scope.

### 8.1 Task file lifecycle

Roadmap-derived task files live in the roadmap task folders and are part of the workflow contract.

- Active roadmap task files belong in `roadmap/pending_tasks/`.
- Task file names SHOULD begin with the roadmap PR ID so the backlog entry, task file, and implementation PR stay linked.
- When a task is completed and a PR is created to land that work, the associated task file MUST be moved from `roadmap/pending_tasks/` to `roadmap/completed_tasks/` in that same PR.
- Do not leave completed task files in `roadmap/pending_tasks/`.
- Do not duplicate a finished task in both folders. Move it; do not copy it.

### 8.2 Source-of-truth by artifact type

Use the right file as the right kind of truth.

- `roadmap/wrela_v0_pr_backlog.json` is the machine-readable source of truth for roadmap task scope, dependencies, and ordering.
- `roadmap/wrela_v0_rust_project_plan.md` is the human-readable roadmap companion and dependency narrative.
- `roadmap/pending_tasks/` and `roadmap/completed_tasks/` are the source of truth for execution status.
- `.github/ISSUE_TEMPLATE/*.yml` are the source of truth for issue intake shape and machine-readable issue metadata keys.
- `.github/PULL_REQUEST_TEMPLATE.md` is the source of truth for PR packet shape and machine-readable PR metadata keys.
- `docs/process/process_contract.json` is the source of truth for bootstrap-phase machine-checkable process invariants.
- `docs/process/validate_process_contract.py` is the bootstrap validator for the process contract until `xtask` absorbs that job.
- `docs/process/TRIAGE_GUIDELINES.md` is the source of truth for routing, severity, and task readiness.
- `docs/process/CODE_REVIEW_GUIDELINES.md` is the source of truth for review expectations and blocking criteria.

Do not start speculative work. Do not invent work because it seems neat. Do not add “helpful” architecture that is not on the critical path.

## 9. How to choose the next task

When several tasks are available, choose in this order:

1. the earliest dependency-blocking task,
2. the task that improves harness capability,
3. the task that reduces uncertainty for many later tasks,
4. the task that stays mostly inside one crate,
5. the task that generates new verification artifacts.

Harness work stays ahead of content work.

### 9.1 Bootstrap default

If the repo still lacks the Rust workspace scaffold, do not jump ahead into later runtime tasks.

- When `Cargo.toml`, `xtask/`, and the crate/app tree are absent, the default implementation task is the earliest unblocked pending task that creates that scaffold.
- In the current roadmap ordering, that normally means `PR-000` unless a human explicitly directs otherwise.
- Do not start later crate work by improvising partial local structures that conflict with the planned scaffold.

## 10. Autonomous workflow contract

For every task, follow this flow.

1. Read the task description, dependencies, and acceptance criteria.
2. Identify the smallest mergeable change that satisfies the task.
3. Write or update a short design note if the task changes APIs, crate boundaries, data formats, or determinism behavior.
4. Implement inside the narrowest possible crate boundary.
5. Add tests and artifact generation before claiming success.
6. Run the relevant automation commands.
7. Produce a PR packet with verification instructions and artifact locations.
8. If blocked, escalate with a decision memo instead of widening scope.

This flow is not complete at local green status. Unless a human explicitly requests local-only work, implementation tasks continue through the GitHub PR and review loop in Section 11.

### 10.1 Bootstrap-phase workflow

When the active task happens before the full workspace and harness exist, follow the same discipline with repo-accurate proof.

- Keep bootstrap PRs narrow. Do not mix initial scaffold work with later subsystem implementation unless the task explicitly requires both.
- Validate the strongest checks that actually exist for the current repo state: JSON parsing, path existence, task coverage, template consistency, and document contract consistency.
- State explicitly which normal Cargo or harness verification commands are not yet available and which roadmap task is expected to introduce them.
- As soon as a task creates a new stable surface, document it in the appropriate contract file instead of leaving it implicit.

## 11. PR contract

Every autonomous PR MUST be independently mergeable. It MUST leave the repo in a working state.
If the PR completes a roadmap task represented by a file in `roadmap/pending_tasks/`, that file MUST be moved to `roadmap/completed_tasks/` as part of the same PR.

### 11.0 Branch and GitHub PR workflow

Autonomous implementation work MUST go through a dedicated task branch and a GitHub pull request unless a human explicitly asks for a local-only change.

- Create or switch to the task branch before implementation work, not after local verification.
- Each new implementation task SHOULD start from its own branch.
- Branches SHOULD use the task ID when one exists and SHOULD stay scoped to that task.
- Do not stack unrelated task work on the same branch.
- Do not merge task work directly from the default branch without opening a GitHub PR.
- The GitHub PR is part of the task workflow, not an optional afterthought.
- Do not mark a task complete just because the code compiles or local tests pass. The PR must exist unless the human explicitly waived that requirement.

### 11.1 Required PR contents

Every PR description must include:

- task ID and dependency context,
- purpose,
- exact scope,
- crates touched,
- design choices,
- determinism impact,
- performance impact,
- tuning surfaces added or changed,
- tests added,
- commands run,
- artifacts produced,
- known risks,
- explicit follow-ups.

### 11.2 PR shape rules

- One PR should usually target one subsystem crate plus tests/docs.
- Do not mix a refactor with a feature unless the refactor is the minimum required to land the feature.
- Do not do drive-by cleanup.
- Do not rename broad swaths of code without a written reason.
- Do not reformat unrelated files.
- Soft size target: keep PRs small enough to review quickly. If the change is large, explain why it could not be split.
- A PR is not done because code exists. A PR is done when the acceptance criteria are met and the proof is machine-runnable.

### 11.3 Merge gates

A PR must not be marked complete unless all applicable gates pass:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask verify`
- relevant scenario or capture commands
- relevant perf checks for runtime-affecting work

### 11.4 Local Claude review loop

Autonomous agents MUST run local Claude Code review before merge unless a human explicitly waives that step.

Local verification is necessary but insufficient. The task remains in progress until this review loop is completed or explicitly waived by a human.

- Push the branch when the task work is ready for review, then open or update the GitHub PR.
- Run Claude from the repo root against the current branch versus `origin/main`, using `docs/process/CODE_REVIEW_GUIDELINES.md` as the review policy.
- Wait up to about 2 minutes for the Claude review to return before treating the run as hung.
- Address actionable review findings on the branch, rerun the relevant verification, and push the follow-up commit(s).
- Treat every push as a new review cycle. After a follow-up push, rerun Claude review on the latest branch state.
- If a review suggestion is intentionally not taken, record the reason clearly in the PR.
- If the Claude run hangs or fails, record that explicitly in the PR and do not claim the review succeeded.
- Do not merge immediately after a follow-up push until the latest branch state has gone through the Claude review loop.
- Human review and taste checkpoints still take precedence where this document already requires them.

### 11.4.1 Claude invocation

- Use the repository review policy as input, not a generic review prompt. Pass `docs/process/CODE_REVIEW_GUIDELINES.md` explicitly in the prompt.
- Prefer reviewing the current branch against `origin/main` from the repo root.
- Preferred command shape:

```bash
claude --permission-mode bypassPermissions --dangerously-skip-permissions -p \
"Use docs/process/CODE_REVIEW_GUIDELINES.md as the review policy for this repository.
Review the current git branch against origin/main.
Focus on bugs, behavioral regressions, determinism risks, missing tests, schema or contract gaps, and process violations.
Return findings first, ordered by severity, with file references where possible.
If there are no findings, say that explicitly and mention residual risks."
```

- Empirical note for this repo: direct branch review works better in non-interactive mode than `--from-pr` or very large diff-fed prompts, which may stall.

### 11.5 Bootstrap verification exception

Before the workspace scaffold and its automation surface exist, some normal gates are unavailable. That is only acceptable for the earliest bootstrap tasks.

- If `cargo fmt`, `cargo clippy`, `cargo xtask verify`, or scenario commands do not exist yet, the PR MUST say so explicitly.
- In that phase, the PR MUST still run the strongest available checks relevant to the change.
- Touching roadmap/process/templates in bootstrap mode MUST include consistency checks against the backlog, task folders, or template contracts when applicable.
- Once `PR-000` lands, code-bearing PRs are expected to converge rapidly to the standard Cargo and `xtask` gates. Missing those gates after the scaffold exists is a real failure, not a bootstrap excuse.

## 12. Coding standards

### 12.1 General code style

- Prefer explicit data flow over magic.
- Prefer plain structs and enums over deep trait hierarchies.
- Prefer small, boring functions over clever generic frameworks.
- Avoid macros unless they remove major duplication and stay debuggable.
- Avoid hidden global state.
- Avoid ambient singletons except for clearly documented platform integrations.
- Prefer composition over inheritance-like abstraction patterns.
- Keep public APIs narrow.
- Keep module boundaries obvious.

### 12.2 Unsafe code

Unsafe code is forbidden unless all of the following are true:

- there is a measurable reason,
- the unsafe boundary is tiny,
- the invariants are documented inline,
- tests cover the boundary,
- a human has explicitly approved it.

### 12.3 Error handling

- Runtime code must not panic on ordinary bad data.
- Startup/config errors should fail loudly with clear messages.
- Harness commands must always try to emit a terminal report, even on failure.
- Error values should be structured and inspectable.

### 12.4 Configuration and serialization

- Scenario files, tweak packs, and reports must use stable, diff-friendly serialization.
- Serialized output should have deterministic ordering where practical.
- Any schema change must include backward-compatibility notes and tests.

### 12.5 Toolchain and dependencies

- Prefer stable Rust. Do not require nightly unless there is a written ADR and explicit human approval.
- New dependencies must be justified in the PR. State what problem they solve and why existing workspace code is insufficient.
- Prefer a small number of well-supported crates over framework sprawl.
- Keep versions workspace-managed where practical.
- Do not commit large binary blobs, generated caches, or opaque artifacts as source of truth. Only approved baselines and golden references belong in version control.

## 13. Determinism rules

Determinism is a first-class feature.

- All procedural generation must start from an explicit root seed.
- Subsystems must derive their own sub-seeds. No shared ambient RNG.
- No global random number generator.
- No wall-clock time in gameplay or generation logic.
- Simulation runs on a fixed step. Variable frame time must not change gameplay outcomes.
- Replays must record inputs and configuration, not only outcomes.
- Time sources must be injectable in tests.
- Any unavoidable floating-point tolerance must be documented where it appears.
- If a change weakens determinism, the PR must say so explicitly and justify it.

## 14. Testing strategy

All important code must be testable. If it is hard to test, improve the design.

### 14.1 Required test types

Use the right test for the job.

- **Unit tests** for pure logic and local invariants.
- **Property tests** for generators, geometry, noise, placement, and math invariants.
- **Snapshot tests** for reports, tweak packs, stable structural outputs, and reference payloads.
- **Integration tests** for crate boundaries and subsystem wiring.
- **Headless scenario tests** for gameplay loops.
- **Golden capture tests** for render-affecting work where visual stability matters.
- **Benchmarks** for hot paths and perf-sensitive systems.

### 14.2 Test expectations by subsystem

- `wr_world_seed`, `wr_math`, `wr_world_gen`, `wr_procgeo`, and `wr_combat` should have strong property-test coverage.
- `wr_tools_harness` and `wr_telemetry` should have schema, snapshot, and integration tests.
- `wr_render_*` changes should include offscreen capture smoke coverage where practical.
- `wr_physics`, `wr_ai`, `wr_actor_player`, and `wr_actor_wraith` should have headless scenario coverage.

### 14.3 Testing rule of thumb

If a feature cannot be verified automatically, add a measurable proxy and document the gap.

Manual play is for judgment and taste. It is not primary proof of correctness.

## 15. Canonical seeds and scenarios

Canonical seeds and scenarios are part of the repo contract. Do not casually change them.

### 15.1 Canonical seeds

Use and preserve named seeds for reproducibility. Start with these unless a task requires more.

- `hero_forest = 0xDEADBEEF`
- `duel_focus = 0xC0FFEE01`
- `perf_path = 0xF00DFACE`

If new canonical seeds are added, document why they exist and what they cover.

### 15.2 Canonical scenarios

Preserve and expand a small stable scenario set.

- `scenarios/smoke/startup.ron`
- `scenarios/traversal/hero_path.ron`
- `scenarios/duel/wraith_smoke.ron`
- `scenarios/duel/light_attack_idle.ron`
- `scenarios/duel/heavy_recovery.ron`
- `scenarios/duel/parry_exchange.ron`
- `scenarios/duel/enemy_lunge.ron`
- `scenarios/duel/dash_reposition.ron`
- `scenarios/duel/jump_attack_recovery.ron`
- `scenarios/lookdev/forest_hero.ron`

When adding a new scenario, document what risk it covers.

## 16. Harness and automation requirements

The autonomous agent must not scrape ad-hoc text output to infer success. The harness must expose stable commands and stable report shapes.

### 16.1 Required command surface

The following is the required end-state repo automation surface. Some commands will not exist until their owning tasks land. Until then, treat the missing command as backlog debt to be implemented, not as permission to invent a private replacement contract.

The repo automation surface is:

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

These commands are part of the product. If their behavior changes, update docs, tests, and any calling automation.

### 16.2 Artifact rules

All automation must write artifacts to stable, discoverable paths.

Artifacts include:

- JSON reports,
- replay files,
- capture images,
- metric dumps,
- diff images,
- benchmark outputs,
- structured failure payloads.

Never require the agent to infer artifact paths from random stdout.

### 16.3 Failure reporting

Failures must be classified clearly. Use stable machine-readable categories such as:

- `build_failed`
- `test_failed`
- `scenario_failed`
- `perf_regressed`
- `visual_regressed`
- `runtime_crash`

A failed run should still emit a terminal report if at all possible.

### 16.4 Baseline discipline

- `baselines/` contains intentional reference artifacts, not random leftovers.
- If a baseline changes, the PR must explain why it changed and what behavior is now considered correct.
- Do not refresh snapshots, captures, or golden reports mechanically without reviewing the behavioral difference.

## 17. Tweak registry and look-dev workflow

Look-dev is a first-class workflow, not an afterthought.

### 17.1 Tweak requirements

Meaningful parameters for the following domains must be live-editable and serializable:

- world,
- atmosphere,
- lighting,
- foliage,
- player,
- combat,
- wraith,
- VFX.

Do not bury important look or feel constants in code if they should be tuned.

### 17.2 Pack rules

- Tweak packs must be diff-friendly.
- Packs must be loadable in both the live client and headless scenarios.
- A changed tweak should dirty the minimum necessary subsystem.
- Named presets should exist for hero shots, combat readability, and performance fallback.

### 17.3 Visual change protocol

Any PR that meaningfully changes visuals should:

1. capture before/after images from canonical cameras,
2. report the seed and tweak pack used,
3. include relevant image metrics if available,
4. note whether the change is aesthetic, functional, or both.

Metrics are proxies, not final taste.

## 18. Gameplay-feel workflow

Gameplay feel is important, but agents do not have taste. They need proxies.

### 18.1 Combat design intent

The combat must **always look sick**. That means readable arcs, sharp recovery, satisfying clash moments, clear telegraphs, and strong motion composition. It does **not** mean physically unconstrained chaos.

### 18.2 Required feel proxies

When modifying combat, prefer to measure and track proxies such as:

- average wind-up duration,
- time-to-recovery,
- clash frequency,
- deadlock frequency,
- duel duration distribution,
- player re-engage time after dash,
- number of unreadable overlapping effects,
- hit-confirm consistency under replay.

### 18.3 Combat scenario rule

Any combat PR must name the canonical scenarios it changes and state what should improve or remain invariant.

## 19. World generation rules

### 19.1 Generation pipeline

The world generation pipeline should remain inspectable and deterministic:

`scalar fields -> placement solution -> tree graphs -> procedural meshes/material parameters -> collision`

Do not collapse the whole biome into opaque procedural magic.

### 19.2 Asset policy

Allowed:

- hand-authored math functions,
- tuned parameter packs,
- small LUTs,
- noise tables,
- SDF or procedural primitives,
- generated caches that can be rebuilt.

Forbidden for v0:

- imported static meshes for world, weapon, or enemy,
- imported animation clips,
- prebaked authored environment geometry.

### 19.3 Cache rule

Generated caches are disposable acceleration artifacts. They are never the source of truth.

## 20. Rendering rules

### 20.1 Render stack expectations

The v0 render path is expected to be a forward-oriented outdoor renderer with strong directional lighting, shadow passes, atmosphere integration, foliage-friendly materials, and a post stack. Do not overbuild a general deferred engine because it sounds respectable.

### 20.2 Shader rule

Do not hide core gameplay or generation logic in shaders if it can be implemented and tested on CPU first.

### 20.3 Render verification

Render-affecting changes should provide offscreen capture smoke coverage where practical. If a shader change cannot be directly asserted, add a proxy metric or golden image comparison.

## 21. Performance rules

Performance is a product requirement.

- Target budget: **1080p60 on the Mac M4 development machine** for canonical traversal and duel scenarios.
- Every performance-sensitive subsystem must expose metrics.
- If a PR changes runtime cost, it must say what cost changed and how it was measured.
- Perf regressions must be treated as functional regressions.
- Do not postpone all optimization until the end. Add instrumentation early.
- Prefer the cheapest solution that preserves the look.

## 22. Telemetry and observability

Use structured telemetry everywhere it matters.

- Add `tracing` spans around scenario execution, world generation phases, capture flows, and combat events.
- Emit counters and timers for hot systems.
- Make reports machine-readable first and human-readable second.
- Record git SHA, seed, tweak pack, scenario, and platform metadata in artifacts.

If something happened and the agent cannot explain why, observability is insufficient.

## 23. Research and external references

For major algorithmic work, prefer direct primary references and record them in the relevant docs and module docs.

Important examples for this repo include:

- Hillaire atmosphere and sky work for the outdoor lighting stack.
- Space-colonization tree growth methods for redwood structure generation.
- XPBD-style approaches for cloth and ribbon simulation.

Do not cargo-cult code from secondary blog posts when the primary material is available.

## 24. Anti-patterns

These are forbidden unless a human explicitly overrides them.

- Do not generalize for hypothetical future engine users.
- Do not introduce ECS-heavy indirection where plain data and systems are enough.
- Do not build a content pipeline for imported character assets.
- Do not hide important state in singleton globals.
- Do not replace deterministic code with clever nondeterministic shortcuts.
- Do not mix unrelated cleanup with feature work.
- Do not create macros to avoid writing normal Rust.
- Do not move core logic into shader code just because it is visually adjacent.
- Do not couple the dev UI directly to gameplay truth.
- Do not use async for local simulation code unless there is a real concurrency boundary.
- Do not rewrite working systems without a benchmarked or verified reason.
- Do not mark work done because the live client “looks okay on my machine.”

## 25. Escalation behavior

When blocked, do not thrash.

### 25.1 Mandatory escalation triggers

Escalate with a decision memo when:

- three implementation attempts fail,
- a crate boundary needs to move,
- a task needs broader scope than planned,
- determinism weakens,
- performance regresses materially,
- a visual or feel question cannot be resolved by metrics.

### 25.2 Decision memo format

A decision memo must include:

- the exact problem,
- evidence collected,
- candidate paths,
- the cheapest next experiment,
- risks of each path,
- recommended action.

Do not solve uncertainty by silently widening the task.

## 26. Human checkpoints

Humans own creative direction, milestone approval, and final taste judgments. Agents own implementation, verification, instrumentation, and proposal generation.

Escalate to a human for:

- major art-direction choices,
- combat-feel disputes that metrics cannot settle,
- scope changes,
- unsafe code,
- major architectural boundary changes,
- performance cuts that alter the visual target.

When taste and metrics conflict, escalate. Do not silently optimize the metric.

## 27. Documentation duties

Update docs when the code changes the contract.

The following must stay current:

- `docs/process/` when workflow rules, review rules, task lifecycle rules, or bootstrap operating rules change,
- `docs/process/process_contract.json` and `docs/process/validate_process_contract.py` when bootstrap validation rules change,
- `docs/architecture/` when subsystem boundaries or data flow change,
- `docs/adr/` when a major decision changes,
- `docs/perf-budget.md` when costs shift materially,
- scenario catalog when canonical scenarios are added or changed,
- seed catalog when canonical seeds are added or changed,
- tuning docs when new tweak surfaces are exposed,
- `.github/ISSUE_TEMPLATE/` and `.github/PULL_REQUEST_TEMPLATE.md` when machine-readable issue or PR contracts change.

## 28. Definition of done

A task is done only when all of the following are true:

- the requested behavior exists,
- the acceptance criteria are met,
- the code is testable,
- tests or verification proxies were added,
- relevant artifacts are produced,
- determinism impact is documented,
- performance impact is documented,
- new tuning surfaces are exposed or intentionally omitted with reason,
- docs are updated where the contract changed,
- the PR is independently mergeable.

## 29. First principles to remember

This project will succeed by being narrow, reproducible, and inspectable.

Do the obvious thing first.
Keep the seams clean.
Make the agent’s job measurable.
Cheat shamelessly for style.
Do not confuse engine ambition with progress.
Ship the slice.
