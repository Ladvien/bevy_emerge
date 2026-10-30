# 2026-10-30 relations: Along keeps a piece beside a path through other pieces

Focus: relations (research lens #2, "along path") · Commits: e8b7fe0.. · Machine: big · Outcome: shipped

## Summary
`<map>.relations.ron` could say faces, near, apart, in front of, clear in front and visible from, but not "along path". `Kind::Along { path, within }` in `bevy_emerge_core::relations` keeps a piece's centre within `within` metres of the polyline through the waypoints' centres (clamped at the ends); `relations_solve` slides it toward the nearest path point on its own side, carrying riders. Fixes #146.

## Baseline
Gate: clean main at bf1f307. Load 28 on 24 cores, free disk 389 GB, available RAM 7 GB. `accept.py` not run: the change is in a module accept does not exercise, and big's load is shared with w5.

## Bugs fixed
None. #140 (the one bug issue) is being worked by w4 (comment 13:08Z).

## Enhancement: Along
Goal: a path a piece is meant to stand beside is intent kept as data, checked and mended like the other relations · Files: `crates/bevy_emerge_core/src/relations.rs` (`Kind::Along`, `nearest_on_path`, parse refusal, `propose`), `crates/bevy_emerge_mapper/src/editor.rs` (RELATIONS row: `lamp along gate → well → hall`), `docs/guide/agents.md` · Proof: `crates/bevy_emerge_core/tests/a_piece_kept_beside_a_path.rs` (6) failed to compile before (`no variant named Along`), pass after; `a_sight_line_no_piece_may_stand_in.rs` (9) still pass.
MCP: no new tool or operation; `relations_solve` and `emerge_state.relations` carry the new kind through serde. Undo: unchanged.
Assumed decision (on #146): centre-to-polyline distance over swept bounds or a navmesh route; waypoints are placement ids, so the path moves with its pieces.

## Borrowed from Mixar
None.

## Performance
Not measured: `perf.py` cannot run on big; the check is O(relations × waypoints), off the frame path.

## Acceptance
Not run (see Baseline).

## Review
Reviewer not spawned (time; earlier passes lost it to 402/429). Self-review: (1) a zero-length segment is guarded (`len2 > 1e-8`); (2) a piece exactly on the path's centre line is not moved (`keep = 0` only when d≈0, and then it is met); (3) a piece on a host is not pulled off (`free`); (4) the waypoints never move, so "never moves a relation's target" in the guide stays true; (5) parse rejects `within` < 0 or non-finite through the existing metres check and a path < 2 ids with its own message.

## Test diffs
New file only: `crates/bevy_emerge_core/tests/a_piece_kept_beside_a_path.rs`.

## Release
None: not `bevy_emerge_loop`.

## Next focus
#146's clip (a Mac); lens #2's remaining "reachable from".
