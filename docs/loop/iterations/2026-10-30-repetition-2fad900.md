# 2026-10-30 repetition: repetition_vary turns the clutter copies that repeat visibly

Focus: repetition (research lens #5) · Commits: 2fad900.. · Machine: big · Outcome: shipped

## Summary
#136/#139 name visible repeats; nothing ended them. `bevy_emerge_core::repetition::vary` turns each clutter copy that looks like an earlier one in range by 180, 90 or 270 degrees until none does, and the registered operation `repetition_vary` (Terrain line `repeats vary`) writes the turned rows with one undo. Fixes #143.

## Baseline
Gate: clean main at e4fd810. `accept.py`: not run (big, shared load 12–36). Free disk 404 GB, available RAM 14 GB.

## Bugs fixed
None found; every open bug issue was claimed by another worker (#140 w4).

## Enhancement: repetition_vary
Goal: one call ends the clutter repeats a player would notice, never structure · Files: `crates/bevy_emerge_core/src/repetition.rs` (`vary`, `Variation`, `TURNS_DEG`, `role_of`), `crates/bevy_emerge_mapper/src/ops_repetition.rs`, `ops.rs` (registry, `UI_ENTRY_POINTS`), `editor.rs` (`vary_repeats`, `rewrite_rows` shared with `solve_relations`), `docs/guide/data-files.md` · Proof: `crates/bevy_emerge_core/tests/vary_turns_clutter_out_of_a_repeat.rs` (4) and `crates/bevy_emerge_mapper/tests/repetition_vary_turns_the_open_maps_repeats.rs` (3) failed before (unresolved `vary` / unknown operation), pass after; `operations_have_two_front_ends`, `relations_solve`, repetition suites 21/21.
MCP: no new tool; `repetition_vary` through generic apply. Undo: inverse `RowDelta` restoring rows by index. Preview: `would`, `would_turn`, `left`, nothing written.
Assumed decision (on #143): turn yaw rather than move apart or swap the descriptor; right-angle turns only, so a footprint stays on its cells.

## Borrowed from Mixar
none

## Performance
Not measured (`perf.py` cannot run on big). Not per-frame: runs on an explicit call. Review P3 fixed: keys built once, distance checked first.

## Acceptance
Not run (load).

## Review
Reviewer ran: no correctness bugs; one P3 (string keys rebuilt per pair), fixed in the follow-up commit.

## Test diffs
New files only; one `mod` line in `tests/all.rs` (owner exception, new editor test file).

## Release
None: not `bevy_emerge_loop`.

## Next focus
Clip for `repeats vary` on a Mac.
