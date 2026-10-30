# 2026-10-30 repetition: repetition_vary never turns a long piece a quarter turn off its cells

Focus: own claim #186 (bug) · Commits: bcb413e, ba103f4, this report · Machine: big · Outcome: shipped (fix only)

## Summary
`repetition::vary` ended a clutter repeat with the first of 180/90/270 degrees that left a copy alike to nothing. `Placed::at` is the centre, so a quarter turn keeps only a square footprint on its cells: a 1 x 2 m bench turned 90 degrees swapped width and depth into the piece beside it. A measured piece whose tipped floor footprint is not square within 1 cm (`SQUARE_TOLERANCE_M`) now takes only the half turn; a copy it cannot end is counted in `Variation::left`. Fixes #186.

## Baseline
The previous w3 pass (128, 14:39–15:10) committed bcb413e and the changelog line, then its push was refused twice (`.git/gate.log`: bcb413e `test rc=101 1198s`, ddf291b `test rc=101 164s`) and the session ended on a model 429 before naming the failing test. This pass merged origin/main (4 docs/media commits, clean) and reran `cargo test --workspace --no-fail-fast` on the merge: rc=0, 96 test binaries `ok`, none failed (`/tmp/w3-gate-test.log`). The earlier red is not reproducible; no OOM kill in `journalctl -k` for 14:40–15:12. `accept.py --source assets`: 10/10. Load 18, free disk 283 GB, available RAM 14 GB.

## Bugs fixed
### #186: repetition_vary turns a long clutter piece a quarter turn
Must-fix: blocks y (an agent's `repetition_vary` moves benches into their neighbours, unchecked), common y (benches, carts, tables are clutter), no workaround n.
Root cause: `TURNS_DEG` assumed a right-angle turn keeps any footprint on its cells; it keeps only a square one. · Regression check: `crates/bevy_emerge_core/tests/vary_never_turns_a_long_piece_off_its_cells.rs` (failed before: three benches turned `[(1, 180.0), (2, 90.0)]`; passes after).
Assumed decision (on #186): half turn only for a long piece, over a quarter turn checked against occupancy (needs the editor) or leaving it.

## Enhancement
None this pass: landing the refused fix was the item.

## Borrowed from Mixar
none

## Performance
Not measured (`perf.py` cannot run on big). Not per-frame: one footprint lookup per placement on an explicit call.

## Acceptance
`reached 10/10` on big, `--source assets`, after; matches the previous big run.

## Review
Reviewer ran in pass 128: no findings. Its note for the owner: a piece smaller than a cell (`site/books`, 0.306 x 0.106 m) also loses its quarter turns, the cautious side of the decision; checking the turned footprint against the cells is the follow-up if wanted.

## Test diffs
One new file, `crates/bevy_emerge_core/tests/vary_never_turns_a_long_piece_off_its_cells.rs`; no existing test changed.

## Release
None: not `bevy_emerge_loop`.

## Next focus
Next unclaimed issue or the focus's headless backlog list.
