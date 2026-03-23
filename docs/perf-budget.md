# Performance Budget

This file records the bootstrap-era geometry budgets that later tasks are expected to preserve or
intentionally revise with evidence.

## Redwood forest batch budget

`PR-017` establishes the first explicit foliage budget for the redwood hero cell.

- Tree foliage hero LOD: at most `6,200` cards, `24,800` logical card vertices, `1` foliage draw
  call.
- Tree foliage mid LOD: at most `5,000` cards, `20,000` logical card vertices, `1` foliage draw
  call.
- Tree foliage far LOD: at most `4,000` cards, `16,000` logical card vertices, `1` foliage draw
  call.
- Forest debug proof: trunk debug geometry plus foliage cards should stay within a two-pass tree
  batch (`debug_geometry` + `foliage_cards`) before later outdoor-render integration changes the
  batching model.

These numbers are intentionally conservative bootstrap contracts. `PR-036` is expected to revisit
them with end-to-end frame measurements and the real outdoor renderer in place.
