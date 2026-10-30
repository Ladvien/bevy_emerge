# 2026-10-30 core: relations::solve mends a map's broken relations (#137 slice 3)

Focus: core · Commits: 4353041 · Machine: big · Outcome: shipped

## Summary
`bevy_emerge_core::relations::solve` takes a map's placements and its `<map>.relations.ron` and nudges the pieces of every unmet relation until it holds: turn to face, step nearer or away, onto the other's forward, or move a corridor intruder sideways. A move is kept only when the unmet count falls, so no relation is mended by breaking another.

## Baseline
Gate: clean tree at b58c3a8. `accept.py`: not run (big under load 9–22; this slice is engine-free core with no editor path). Load 9.8, free disk 434 GB, available RAM 9 GB.

## Bugs fixed
None open in the backlog's bug list for this worker; own claim #137 first.

## Enhancement: relations solver
Goal: an agent states intent, the solver finds the coordinates. · Files: `crates/bevy_emerge_core/src/relations.rs` (`Solved`, `solve`, `propose`, `move_piece`), `crates/bevy_emerge_core/tests/a_solver_mends_broken_relations.rs` · Proof: the test failed before (`unresolved import bevy_emerge_core::relations::solve`), 5 pass after; slice 1's 6 still pass.
Rules: a piece with `on: Some(host)` is never pulled off it (an intruder on a host moves the host); riders keep their offset in the host's frame and turn with it; `on` cycles in a malformed file end (walks bounded by the placement count); a relation naming a missing piece moves nothing.
MCP: none this slice. Next slice applies `solve` through generic apply as one undoable op.

## Borrowed from Mixar
none.

## Performance
Not measured: not run per frame; `solve` is O(rounds × relations² × placements), rounds ≤ 8.

## Acceptance
Not run (see Baseline).

## Review
Reviewer not spawned (this worker's earlier passes saw model 402). Self-review: (1) never worse — enforced by strict decrease; (2) termination — strict decrease plus 8 rounds, bounded rider walk; (3) human work — solve returns new placements, writes nothing; (4) conventions — forward `(sin yaw, cos yaw)` and `local` frame reused from `check`; (5) gap — moves are not snapped to the grid and do not test collisions beyond the relations given.

## Test diffs
New file `a_solver_mends_broken_relations.rs`; no existing test changed.

## Release
None: not `bevy_emerge_loop`.

## Next focus
#137 slice 4: apply `solve` as one undoable op over generic apply.
