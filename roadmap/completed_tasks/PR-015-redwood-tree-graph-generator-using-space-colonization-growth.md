# PR-015 - Redwood tree graph generator using space-colonization growth

- Status: completed
- Category: Roadmap Task
- Lane: World
- Depends on: PR-014, PR-008
- Source: `roadmap/wrela_v0_pr_backlog.json`
- Source PR backlog ID: `PR-015`

## Primary Crates
- wr_world_gen
- wr_math

## Scope
Generate the branching skeletons for hero redwoods from seed and growth parameters.

## Requirements
- Implement a tree graph generator inspired by the space-colonization algorithm, with redwood-specific biases for tall trunks, sparse lower limbs, and elevated canopy mass.
- Expose parameters for attraction radius, segment length, tropism, taper, branch culling, and canopy envelope.
- Output graph-only data first, not render meshes.

## Acceptance Criteria
- Tree graphs are acyclic and connected.
- Radius/taper decreases monotonically away from the trunk root except where explicitly overridden for buttress behavior.
- Canonical seeds produce believable redwood silhouettes in graph debug renders.

## Verification
- Property tests for graph invariants.
- Snapshot tests for graph stats and selected node lists.
- Benchmarks for batch generation of tree graphs.

## References
- [Modeling Trees with a Space Colonization Algorithm](https://algorithmicbotany.org/papers/colonization.egwnp2007.pdf) - Primary tree-graph generation reference.
