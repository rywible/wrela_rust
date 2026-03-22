# Triage Guidelines

## Purpose

This document defines how work is classified, routed, prioritized, and escalated in this repository.

Its goals are to:

- keep autonomous work aligned with roadmap intent,
- prevent scope drift and duplicate effort,
- make issue routing predictable,
- separate correctness problems from taste/tuning work,
- ensure blockers are escalated quickly instead of silently churned on.

These rules apply to backlog tasks, bugs, regressions, tuning requests, architectural decisions, and PR review follow-up.

## Core Principles

1. Triage is a routing function, not an implementation discussion.
2. Prefer narrow, mergeable work over broad tickets.
3. Route by owning subsystem first, then by urgency.
4. Correctness, determinism, and broken tooling beat feature work.
5. Visual taste and combat feel are important, but should not preempt broken build/test/replay infrastructure unless explicitly approved.
6. If an item cannot be verified, it is not triage-ready.
7. If an item is really a decision, convert it to an ADR instead of leaving it as an open-ended task.

## Triage Categories

Every issue should belong to exactly one primary category.

### 1. Roadmap Task

Use for planned implementation work that advances the approved project plan.

Examples:

- add deterministic replay capture,
- implement procedural redwood trunk generator,
- add sword trajectory family for light attack,
- implement atmosphere LUT generation,
- add screenshot regression runner.

### 2. Bug / Regression

Use for behavior that is incorrect relative to an accepted baseline.

Examples:

- replay diverges from baseline seed,
- frame time doubles in canonical forest scene,
- enemy controller no longer exits recovery,
- atmosphere shader produces NaNs on Metal,
- canonical screenshot changed unexpectedly.

### 3. Lookdev / Combat Tuning

Use for changes where the system exists, but output quality needs adjustment.

Examples:

- canopy density reads too noisy,
- fog color separation is weak,
- heavy attack recovery feels sluggish,
- dash distance overshoots readable duel spacing,
- wraith silhouette loses clarity at mid distance.

These should not be used to hide missing systems. If the system does not exist or is structurally wrong, route as a roadmap task or bug.

### 4. ADR / Decision

Use when progress depends on choosing between meaningful alternatives.

Examples:

- CPU vs GPU ownership for a generation stage,
- whether combat hit resolution stays kinematic or gains constrained contact locks,
- whether a subsystem boundary should move between crates,
- whether to accept a controlled nondeterministic optimization path.

If discussion exceeds one implementation loop without convergence, escalate to ADR.

## Severity

Severity describes impact, not effort.

### S0 — Repository blocked

Use when the project cannot make meaningful progress.

Examples:

- default branch does not build,
- tests cannot run in CI,
- harness cannot execute scenarios,
- deterministic replay system is broken globally,
- rendering boot fails on target dev hardware.

**Target response:** immediate.  
**Priority rule:** preempts all non-S0 work.

### S1 — Critical milestone risk

Use when a core v0 capability is broken or missing in a way that threatens the vertical slice.

Examples:

- combat loop cannot complete canonical duel,
- forest scene exceeds performance budget by a major margin,
- hero scene visuals are broken in a way that invalidates comparisons,
- agent harness cannot verify gameplay loop claims,
- world generation is nondeterministic for canonical seeds.

### S2 — Important but containable

Use for substantial issues that hurt progress or quality but do not fully block milestone movement.

Examples:

- one subsystem lacks expected instrumentation,
- one enemy behavior path is unstable,
- one visual preset regressed,
- a crate boundary is getting muddy,
- a tuning surface is missing serialization.

### S3 — Normal

Use for routine implementation, quality improvement, or localized cleanup that supports the roadmap.

Examples:

- add new canonical seed,
- improve inspector clarity,
- refine a trajectory family,
- expand property tests,
- reduce minor perf overhead in a non-critical pass.

### S4 — Deferred / speculative

Use for ideas that may be valuable later but are not currently justified.

Examples:

- generalize runtime into framework abstractions,
- add streaming world support,
- add dynamic time-of-day,
- explore destruction,
- speculative rendering optimization without current bottleneck evidence.

S4 items should usually live in a parking lot or future milestone, not the active backlog.

## Priority

Priority describes execution order inside a severity band.

- **P0** — do now
- **P1** — next up
- **P2** — planned, not immediate
- **P3** — backlog / defer

Default triage order is:

1. S0/P0
2. S1/P0
3. S1/P1
4. S2/P0
5. S2/P1
6. S3/P1
7. S3/P2
8. S4/P3

When severity and roadmap value conflict, use this decision order:

1. broken build / broken CI / broken harness
2. determinism / replay correctness
3. performance budget regressions
4. canonical gameplay regressions
5. canonical visual regressions
6. roadmap feature implementation
7. quality-of-life improvements
8. speculative future work

## Ownership Routing

Each item must name one primary owning area.

- **core** — shared types, config, deterministic primitives, IDs, utilities
- **harness** — automation daemon, scenario execution, capture, orchestration, machine-readable verification
- **render** — frame graph, GPU passes, materials, post, visibility, debug overlays
- **sky** — atmosphere, sun, fog, aerial perspective, sky LUTs, hero-lighting pipeline
- **worldgen** — terrain, ecology rules, redwood growth, understory placement, canonical seed generation
- **combat** — verbs, trajectories, timing windows, recovery, hit events, duel rules
- **actors** — wraith behavior, silhouette presentation, cloth/ribbon state, telekinetic blade control
- **physics** — collision, queries, kinematic constraints, integration wrappers
- **tools** — tweak UI, inspectors, preset management, profiling surfaces, visual diff workflow
- **tests_support** — fixtures, baselines, golden references, scenario catalog, comparison helpers
- **docs/process** — workflow rules, ADRs, roadmap hygiene, developer process

If an issue spans more than one area, assign one primary owner and list secondary areas.  
Do not create “shared ownership” with no decider.

## Triage Readiness Checklist

An item is triage-ready only if it includes enough detail to route and verify.

### Required for all issues

- clear problem statement,
- category,
- expected behavior or target outcome,
- owning area,
- evidence or rationale,
- proposed verification path.

### Required for bugs/regressions

- baseline that regressed,
- reproduction steps or replay scenario,
- seed/config/build info where relevant,
- observed vs expected behavior,
- impact statement.

### Required for tuning issues

- canonical scene or duel scenario,
- what feels or looks wrong,
- what should improve,
- measurable proxy if available,
- before/after captures if the issue is based on a recent regression.

### Required for roadmap tasks

- dependency context,
- acceptance criteria,
- what crates should be touched,
- test plan,
- any required docs or harness updates.

### Required for ADRs

- decision to be made,
- options considered,
- tradeoffs,
- recommendation if available,
- blocking impact.

If an item lacks this information, request clarification or convert it into a draft/specification instead of pushing it into active implementation.

## Duplicate and Overlap Handling

When a new issue overlaps an existing one:

- keep the older canonical issue if it still reflects the problem,
- close duplicates with a reference to the canonical issue,
- merge useful evidence into the canonical issue,
- do not split one bug into many tickets unless independent fixes can land separately.

When a single issue contains multiple unrelated problems:

- split by verification path and owning area,
- preserve a parent tracking issue only if coordination actually matters.

## Converting Between Issue Types

Triage may reclassify items.

### Convert to Bug / Regression when:

- accepted behavior stopped working,
- a baseline artifact changed unexpectedly,
- performance or determinism regressed.

### Convert to Tuning when:

- the system works structurally,
- the problem is feel/readability/quality,
- the required work is parameter or local-behavior adjustment.

### Convert to Roadmap Task when:

- functionality is missing,
- implementation is required to advance v0,
- the work is not a regression from an accepted baseline.

### Convert to ADR when:

- progress is blocked by a design choice,
- discussion keeps reopening architecture questions,
- multiple valid paths exist and implementation should pause pending a decision.

## Escalation Rules

Escalate immediately when any of the following are true:

- the default branch does not build or test,
- deterministic replay diverges in canonical scenarios,
- harness automation cannot verify key milestones,
- target hardware no longer reaches agreed baseline performance in hero scenes,
- a subsystem change breaks crate ownership boundaries significantly,
- an agent has attempted the same class of fix repeatedly without measurable progress,
- a change appears to improve metrics while clearly harming feel or readability.

### Agent escalation behavior

When blocked, the agent must not churn indefinitely.  
It should open or update an ADR / triage note with:

- root issue,
- current evidence,
- attempted paths,
- why they failed or remain inconclusive,
- cheapest next experiment,
- whether human taste judgment is required.

## Fast-Track Rules

The following can bypass normal backlog order:

1. Broken build / CI / harness startup
2. Replay determinism failures
3. Performance regressions in canonical benchmark scenes
4. Regressions in merge-gating golden artifacts
5. Issues that block multiple in-flight PRs

Fast-tracked items should still be scoped narrowly.  
Do not smuggle unrelated cleanup into them.

## Milestone Alignment

Triage should map active work to the current milestone.  
For v0, active work must support:

- autonomous harness and environment setup,
- deterministic world generation,
- fixed hero sky/lighting,
- playable first-person floating-katana duel loop,
- one procedural wraith archetype,
- one beautiful procedural redwood forest slice,
- testable and replayable verification path,
- target performance on M4 at 1080p60.

If an issue does not materially support this milestone, it should usually be downgraded, deferred, or parked.

## Definition of Ready for Assignment

Before assigning an issue for implementation, confirm:

- category is correct,
- ownership is clear,
- dependencies are identified,
- AC exists,
- verification path exists,
- issue scope fits one mergeable PR or an intentional small PR sequence,
- no unresolved architectural decision is hidden inside the task.

## Definition of Done for Triage

A triaged issue is considered properly triaged when:

- category is assigned,
- severity and priority are assigned,
- owner is assigned,
- milestone status is clear,
- dependencies/blockers are recorded,
- next action is obvious,
- issue type and verification path match the actual work.

## Referencing and Cross-Linking Rules

Use repository documents as the source of truth for process and scope.

Recommended references:

- `AGENTS.md` — top-level operating contract
- `docs/process/process_contract.json` — machine-checkable bootstrap process invariants
- `docs/process/validate_process_contract.py` — bootstrap process validator
- `docs/process/TRIAGE_GUIDELINES.md` — issue routing and prioritization
- `docs/process/CODE_REVIEW_GUIDELINES.md` — review standards and merge expectations
- `.github/PULL_REQUEST_TEMPLATE.md` — PR evidence contract
- `.github/ISSUE_TEMPLATE/*.yml` — intake structure for work items
- roadmap / architecture docs — implementation intent and subsystem boundaries
- ADR documents — architectural decisions and tradeoffs

### Linking guidance

- `AGENTS.md` should link to the process docs and templates as the primary index.
- Process docs should link back to `AGENTS.md` as the governing contract.
- PRs should link the issue, relevant ADRs, and any changed process documents.
- Issues should link the roadmap item or milestone they support.
- ADRs should link the affected crates and superseded decisions.

Avoid duplicating process rules across many files. Keep the short form in `AGENTS.md`, and put detailed procedure in the docs it links to.
