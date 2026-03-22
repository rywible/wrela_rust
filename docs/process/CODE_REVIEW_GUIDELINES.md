# Code Review Guidelines

These guidelines define how code review works in this repository.

This is not a normal repo. It is an autonomous-first build experiment. That means review is not just about catching bugs or style nits. Review is the control surface that keeps the project from drifting out of scope, losing determinism, or shipping something that is technically green and creatively dead.

Reviewers are guardians of direction, not passive approvers.

## 1. What review is for here

Every review should answer five questions in this order:

1. Should this work exist right now?
2. Is the scope narrow enough to merge safely?
3. Is the proof strong enough to trust it?
4. Does the implementation respect crate boundaries and repo laws?
5. Does it improve the game or harness without creating hidden debt?

If the answer to any of the first three is "no," stop there and block the PR.

## 2. Review hierarchy

When tradeoffs conflict during review, use this order:

1. deterministic correctness,
2. mergeable simplicity,
3. observability and testability,
4. frame-time stability,
5. visual quality and combat feel,
6. future generality.

Do not ask for a more abstract design unless it clearly improves one of the first four.

## 3. Review starts before reading code

Read the PR body first.

A PR is not ready for serious review unless it has all of these:

- a named task ID,
- dependency context,
- explicit scope,
- acceptance criteria mapping,
- commands run,
- tests added or updated,
- artifact paths,
- determinism notes,
- performance notes when relevant,
- tuning / visual / combat packets when relevant,
- explicit risks and follow-ups.

If the PR body is weak, incomplete, or hand-wavy, send it back before spending time on the diff.

## 4. Hard blocking reasons

Any one of these is enough to block a PR.

### 4.1 Scope and roadmap failures

Block if:

- the work is not tied to a named task, issue, or explicit human request,
- the PR silently widens v0 scope,
- the change bundles several unrelated concerns,
- the PR is not independently mergeable,
- the implementation performs a hidden rewrite under the cover of a small task.

### 4.2 Evidence failures

Block if:

- acceptance criteria are not mapped to proof,
- the proof depends mostly on manual play,
- the PR changes hot paths without measurements,
- the PR changes visuals or combat feel without a before/after packet,
- the PR changes baselines without explaining why.

### 4.3 Determinism and replay failures

Block if:

- randomness is introduced outside explicit seed streams,
- time is pulled from ambient system APIs instead of injectable clocks,
- system ordering becomes unstable without being documented and tested,
- replay compatibility changes without being stated,
- a convenience helper hides nondeterminism.

### 4.4 Architecture failures

Block if:

- gameplay state leaks into render ownership,
- crate responsibilities become fuzzy,
- `wr_game` starts absorbing subsystem logic,
- a new top-level crate is introduced without a strong reason,
- a subsystem self-registers or hides integration instead of exposing explicit init/plugin entrypoints.

### 4.5 Maintenance and debugging failures

Block if:

- the code is hard to inspect or replay,
- logging or metrics were removed without replacement,
- the change adds deep macro magic with little payoff,
- the PR adds a dependency without a clear reason and ownership story.

## 5. What reviewers should optimize for

Reviewers should push the repo toward:

- narrow mergeable slices,
- explicit data flow,
- deterministic state transitions,
- good reports and artifacts,
- stable scenario verification,
- strong subsystem seams,
- cheat-friendly but testable implementations.

Do not optimize for theoretical elegance at the cost of shipping.

## 6. Review workflow

Use this order. It is intentionally strict.

### Step 1: Legitimacy

Check that the PR should exist.

- Does it map to a named task?
- Is it the smallest mergeable slice?
- Does it stay inside the locked v0 mission?
- Does it avoid speculative generalization?

### Step 2: Proof packet

Check the PR body and artifacts.

- Are all acceptance criteria mapped to proof?
- Are commands concrete and machine-runnable?
- Are artifact paths stable and repo-relative?
- Are scenario names, seeds, and tweak packs named explicitly?

### Step 3: Boundary integrity

Check the architecture.

- Did logic stay in the right crate?
- Are new interfaces explicit and small?
- Did the PR increase cross-crate coupling?
- Did it touch `wr_game` or app shells more than the task really needed?

### Step 4: Determinism and replay

Check the non-obvious hazards.

- Is every random branch tied to a named RNG stream?
- Are update orders stable?
- Are clocks injectable?
- Does serialization stay stable?
- Can the scenario still replay from seed + input trace?

### Step 5: Performance and observability

Check whether the system can be trusted in the future.

- Are metrics exposed?
- Are performance claims measured?
- Is the cost change acceptable for the current budget?
- Can the harness detect regressions next time?

### Step 6: Domain-specific quality

Only after the first five steps should you spend time on the actual domain details.

## 7. Domain-specific review checklists

### 7.1 Harness, tooling, and infra

Focus on:

- stable command surfaces,
- predictable artifact paths,
- machine-readable output,
- scenario discoverability,
- failure modes that explain themselves,
- idempotent automation.

Block if the harness requires human interpretation to know whether something passed.

### 7.2 World generation and procedural geometry

Focus on:

- seed ownership,
- deterministic sub-seeding,
- structural invariants,
- property tests for geometry bounds and topology,
- cacheability and serialization,
- narrow ownership between `wr_world_gen` and `wr_procgeo`.

Push back on clever generation code that cannot be tested headlessly.

### 7.3 Rendering and atmosphere

Focus on:

- render/sim separation,
- extracted immutable scene data,
- pass and resource lifetime clarity,
- shader inputs that are testable or mirrored on CPU where necessary,
- performance counters and stable hero-scene captures,
- image-diff or proxy metrics when a visual baseline changes.

Do not reject visual cheats just because they are cheats. Reject them if they are opaque, unstable, or impossible to verify.

### 7.4 Combat and AI

Focus on:

- the verb -> move family -> solver -> clash/recovery chain,
- deterministic scenario outcomes,
- bounded tuning surfaces,
- scenario packets for player and enemy behavior,
- readability of state changes,
- preservation of authored-feeling motion.

Push back on freeform simulation creep. The goal is expressive, authored-feeling math, not unconstrained rigid-body fencing.

### 7.5 Physics and collision

Focus on:

- deterministic wrappers and query boundaries,
- fixed-step assumptions,
- stable contact semantics,
- minimal surface area exposed to higher-level systems,
- clear conversion rules between physics data and gameplay events.

### 7.6 Visual and combat taste review

When `affects.visuals = yes` or `affects.combat_feel = yes`, there must be a human taste review. The reviewer is not asked to replay the full game. They are asked to judge a bounded packet.

For visuals, require:

- seed,
- scenario,
- camera set,
- tweak pack,
- before captures,
- after captures,
- proxy metrics,
- a statement of what a human should judge.

For combat, require:

- canonical scenario list,
- verbs affected,
- before and after timings or other feel proxies,
- preserved invariants,
- a statement of what a human should judge.

If the PR says a feel or visual improvement happened but cannot show a packet, block it.

## 8. Comment taxonomy

Use clear prefixes so autonomous tooling and humans can read review intent.

- `BLOCKING:` Must be resolved before merge.
- `IMPORTANT:` Strong concern, but not necessarily blocking if addressed with explanation.
- `QUESTION:` Something unclear that needs an answer.
- `SUGGESTION:` Optional improvement.
- `NIT:` Small local cleanup or wording issue.
- `PRAISE:` Call out a strong pattern worth repeating.

Do not bury blocking concerns inside a paragraph of mixed feedback.

## 9. What not to nitpick

Do not waste review energy on these unless they materially affect the hierarchy.

- personal formatting preferences already covered by tooling,
- harmless local naming choices,
- minor style quirks in internal code,
- the presence of a tasteful cheat that stays testable,
- the lack of future-facing abstraction for hypothetical engine reuse.

This repo is not trying to impress architecture judges. It is trying to ship a sharp vertical slice under autonomous execution.

## 10. Anti-patterns reviewers should push back on

Push back hard on:

- speculative abstraction,
- broad refactors mixed into feature work,
- hidden changes to crate ownership,
- large diffs with weak proofs,
- baseline churn without explanation,
- removing observability to simplify code,
- using async where a simpler synchronous path would do,
- moving logic into shaders before a CPU-verifiable model exists,
- silent dependency creep,
- replacing explicit data with trait-heavy indirection for no immediate gain.

## 11. Approval rules

A reviewer should approve when all of these are true:

- the PR belongs in the roadmap sequence,
- scope is tight and mergeable,
- proof is adequate for the claims,
- crate boundaries remain clean,
- determinism and replay risks are handled,
- performance impact is measured or explicitly bounded,
- all required human taste review has happened,
- known risks are acceptable for the current milestone.

Do not withhold approval because the system could be prettier in some abstract future. Approve the mergeable slice that moves the project forward.

### 11.1 Automated reviewer loop

When review is automated on GitHub, the expected operating loop is:

1. author pushes reviewable work,
2. reviewer runs,
3. author addresses feedback and pushes again,
4. reviewer runs again,
5. loop continues until the latest push gets an explicit green-light signal, such as `thumbs up`.

Do not treat an old approval signal as valid after a new push if the automated reviewer is expected to rerun.
The final merge decision should be based on the latest reviewed state.

## 12. Escalation rules for reviewers

If you discover a problem that should change architecture, scope, or dependency ordering, do not try to redesign the PR in comment threads.

Instead:

1. block the PR,
2. state the minimal blocking reason,
3. ask for a follow-up decision / ADR issue if needed,
4. keep the current review focused on whether this PR should merge.

Comment threads are a bad place to invent new architecture.

## 13. Quick review checklist

Use this when you need the fast path.

- [ ] PR maps to a named task.
- [ ] Scope is small and independently mergeable.
- [ ] Acceptance criteria are mapped to proof.
- [ ] Commands, tests, and artifact paths are present.
- [ ] Determinism / replay notes are adequate.
- [ ] Performance notes are present if the hot path changed.
- [ ] Crate boundaries still make sense.
- [ ] Visual / combat packet exists if relevant.
- [ ] Baseline changes are explained.
- [ ] Follow-ups are explicit instead of hidden in reviewer imagination.

If any unchecked item is important to the task, the PR is not ready.
