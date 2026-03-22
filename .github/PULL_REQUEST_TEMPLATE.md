<!--
Keep every heading. Use `N/A` when not applicable.
Do not delete the metadata block. Keep keys stable so agents and tooling can parse PR bodies.
A PR is not ready until it includes machine-runnable verification and explicit evidence for acceptance criteria.
-->

# Pull Request

> This repository is autonomous-first. A PR without scoped intent, verification evidence, and artifact paths is not ready.
>
> Leave all headings in place. Fill unused sections with `N/A`.

## PR metadata

```yaml
task_id:
task_title:
task_source: backlog|issue|human
depends_on: []
blocked_by: []
pr_kind: feature|refactor|infra|docs|perf|baseline-update|bugfix
status: draft|ready
scope_lock_exception: none
crate_scope: []
app_scope: []
affects:
  determinism: no
  performance: no
  replay: no
  tuning: no
  serialization_schema: no
  visuals: no
  combat_feel: no
  baselines: no
  docs_contract: no
requires_human_taste_review: no
canonical_seed:
canonical_scenarios: []
tweak_pack:
artifact_paths: []
```

## Merge thesis

<!-- One sentence. Why should this merge now? -->

## Dependency context

- **Roadmap / issue link:**
- **Why this task is unblocked now:**
- **What this PR unblocks next:**
- **Why this was not split further:**

## Scope

### In scope

-

### Explicitly out of scope

-

### Non-goals

-

## Crates and key files touched

<!-- Group by crate. Keep this concrete. -->

- `crate_or_app_name`
  - `path/to/file.rs`: why it changed
  - `path/to/other_file.rs`: why it changed

## Design notes

### Problem being solved

### Approach taken

### Important design choices

### Alternatives considered and rejected

### Boundary / API changes

<!-- Note crate boundary changes, data contract changes, plugin/init changes, report schema changes, scenario format changes, etc. -->

## Acceptance criteria mapping

<!-- Copy the task AC here and prove each one. One bullet per AC. -->

- **AC:**
  - **Evidence:**
  - **Proof type:** unit test | property test | integration test | scenario | capture | benchmark | snapshot | manual note
  - **Artifact / test path:**
  - **Notes:**

- **AC:**
  - **Evidence:**
  - **Proof type:**
  - **Artifact / test path:**
  - **Notes:**

## Verification

### Commands run

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask verify
```

<!-- Add every task-specific command actually run. Include scenario, lookdev, capture, replay, perf, or benchmark commands. -->

```bash
# task-specific verification
```

### Tests added or updated

-

### How to reproduce the main verification in under 5 minutes

1.
2.
3.

### Artifacts produced

<!-- Use stable repo-relative paths. Prefer reports/, baselines/, tweak_packs/, or scenario paths over screenshots pasted into comments. -->

- `path/to/artifact.json`: what it proves
- `path/to/capture.png`: what it proves
- `path/to/report.md`: what it proves

## Determinism and replay

- **Root seed(s) used:**
- **Sub-seed / RNG stream changes:**
- **Fixed-step / timing changes:**
- **Replay compatibility impact:**
- **Known nondeterminism risks:**
- **Why this remains reproducible:**

<!-- If this touches generation, simulation, ordering, time sources, floating-point assumptions, or replay file formats, be explicit. -->

## Performance

- **Expected runtime cost change:**
- **Measurement method:**
- **Scenarios measured:**
- **Before:**
- **After:**
- **Budget impact:**
- **Mitigations / fallback path:**

<!-- Do not write “N/A” if the PR materially changes hot paths, render passes, generation complexity, capture flows, or scenario runtime. -->

## Tuning surfaces

- **Knobs added or changed:**
- **Default values chosen:**
- **Tweak pack changes:**
- **Why these are exposed live:**
- **Why any important constant remains hard-coded:**

## Visual review packet

<!-- Required when `affects.visuals = yes`. -->

- **Seed:**
- **Scenario / camera set:**
- **Tweak pack:**
- **Before artifacts:**
- **After artifacts:**
- **Image diff / proxy metrics:**
- **What changed aesthetically:**
- **What should a human reviewer judge:**

## Combat review packet

<!-- Required when `affects.combat_feel = yes`. -->

- **Canonical scenarios impacted:**
- **Player verbs affected:**
- **Enemy behaviors affected:**
- **Feel proxies measured:**
- **Before:**
- **After:**
- **Invariants preserved:**
- **What should a human reviewer judge:**

## Schema, baseline, and artifact contract changes

- **Schema changes:**
- **Baseline files changed:**
- **Why baseline updates are intentional:**
- **Backward-compatibility notes:**
- **Any required follow-up migration:**

## Dependencies added or changed

- **Dependency:**
- **Why it is needed:**
- **Why existing workspace code or dependencies were insufficient:**
- **License / maintenance concerns:**

## Risks and unresolved edges

### Technical risks

-

### Integration risks

-

### Performance risks

-

### Visual / feel risks

-

### What could still fail after merge

-

## Follow-ups

- **Immediate follow-ups unlocked by this PR:**
- **Intentional deferrals:**
- **Suggested next task ID(s):**

## Documentation updated

- [ ] `docs/architecture/` updated if subsystem boundaries or data flow changed
- [ ] `docs/adr/` updated if a major decision changed
- [ ] `docs/perf-budget.md` updated if costs shifted materially
- [ ] scenario catalog updated if canonical scenarios changed
- [ ] seed catalog updated if canonical seeds changed
- [ ] tuning docs updated if new tweak surfaces were exposed
- [ ] no documentation update was required

## Review focus

<!-- Tell the reviewer where to spend attention. Keep this pointed. -->

-

## Automated review follow-up

- **Automated review expected on GitHub:** yes | no
- **Latest automated review status:** pending | feedback received | thumbs-up | none
- **Waited for automated review after each push:** yes | no
- **Automated review artifacts checked:** comments | review summary | reactions | none
- **Follow-up pushes made from automated review:** 0 | 1 | 2+
- **Latest green-light signal on the final push:** yes | no
- **If no follow-up changes were made, why:**

## Merge checklist

- [ ] This PR maps to a named task and states dependency context.
- [ ] Scope is explicit and does not silently widen v0.
- [ ] This PR is independently mergeable.
- [ ] This work lives on a dedicated task branch unless a human explicitly approved an exception.
- [ ] A GitHub PR was opened for this task before merge.
- [ ] Automated GitHub review had time to run after each push.
- [ ] Any actionable automated review feedback was addressed or explicitly declined with reason.
- [ ] The latest push received an explicit automated green light such as `👍`, or a human explicitly waived that requirement.
- [ ] No unrelated cleanup, broad reformatting, or speculative abstraction was included.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] `cargo xtask verify` passes.
- [ ] Task-specific verification commands were run and listed.
- [ ] Acceptance criteria are mapped to concrete evidence.
- [ ] Artifact paths are stable and included.
- [ ] Determinism impact is documented.
- [ ] Performance impact is documented.
- [ ] Tuning surface changes are documented.
- [ ] Replay / scenario impact is documented.
- [ ] Baseline or schema changes are documented and intentional.
- [ ] Required docs were updated, or the absence of doc changes is justified.
- [ ] If visuals changed, before/after captures are included.
- [ ] If combat changed, impacted canonical scenarios and feel proxies are included.
- [ ] If a new dependency was added, justification is included.
- [ ] If human taste review is required, that request is explicit.
- [ ] Known risks and follow-ups are listed.

## Reviewer quick verdict

<!-- Keep this concise so a reviewer can answer fast. -->

- **Safe to merge now:** yes | no
- **Needs human taste review before merge:** yes | no
- **Follow-up required immediately after merge:** yes | no
