# 2026-10-27 editor: a palette click is tested, and E-003's paperwork is on `main` (E-041)

Focus: the palette click path · Commits: `b35dcca..203fbc3` · Machine: laptop ·
Outcome: shipped

## Summary

Two things this pass: the report and LOG line `8eecbe0` landed without (E-003), and E-041's
remaining gap — `editor::on_row_click`, the primary way a person arms a piece, was unreachable
from the suite. `editor::click_palette_row_for_test` fires the same `Activate` the button sends, so
the row's own index, the filter blur, and the `Option` resources are all exercised.

## Baseline

Gate at `b35dcca`: clippy `--workspace --all-targets -- -D warnings` **clean** (exit 0,
`/tmp/gate-clippy-b35dcca.log`), `backlog.py check` consistent. Tree dirty with the untracked
E-003 report — that was the first fix.

Load average 5.8–8.9 on 10 cores; two foreign `cargo build --release` jobs in
`/Users/ladvien/bevy_elements` held the machine for the whole pass. Disk 102 GB free.
**This pass burned its budget on compiles**: a workspace test run exceeded 3000 s waiting on the
build lock, and two fresh `CARGO_TARGET_DIR`s were started before the warm
`/Users/ladvien/target/emerge-loop` was checked, which cost a full Bevy rebuild each. The lesson is
in the next-focus line.

## Bugs fixed

None. No open bug reproduced.

## Enhancement: E-041, the palette click is reachable from a test

Goal: the observer that arms a piece from a click is exercised, not just `arm_piece` beneath it.
Files: `crates/bevy_emerge_mapper/src/editor.rs` (one new `pub fn`, no behaviour change),
`crates/bevy_emerge_mapper/tests/a_palette_click_arms_the_piece.rs` (new, 3 tests).

Proof: the three tests pass at `203fbc3` (`3 passed; 0 failed`, 0.93 s). What the hook buys is not
asserted by a failing-before run — the path was *unreachable*, so there was nothing to fail — but
each of the three assertions names a write only the observer makes: the `PaletteRow` index the
click resolves, the one `brush_arm` line with `source: Human`, the tool returning to `Place`, and
`Filters::typing()` going false.

MCP: no operation, tool, undo strategy or schema changed. `brush_arm` is untouched; this is a test
door onto the existing observer.

## Borrowed from Mixar

none.

## Performance

Not measured. The machine ran two foreign release builds throughout; a timing would measure them.

## Acceptance

`accept.py` not run this pass: the budget went to compiles and the workspace gate never got the
lock. The last recorded run on this machine and source is 10/10 (`b35dcca`'s log line); this change
touches one test hook and no product behaviour, so it cannot move `reached`. Recorded as owed.

## Review

Reviewer could not run — the pass was already past budget with the build queue stalled.
Self-review, mine:

1. **Can the hook pass while the observer is broken?** No: it triggers `Activate` on an entity
   carrying `PaletteRow`, which is only spawned by `rebuild_palette`; the observer is the only
   `Activate` handler that reads that component.
2. **Does it skip anything the real click does?** The real click also carries pointer position,
   which nothing in the observer reads. Everything else — index resolution, blur, the Option
   guards, `arm_piece` — is on the path.
3. **Can it pass by clicking nothing?** No: it panics when the row is absent, and the first test
   asserts the log grew by exactly one `brush_arm` line.
4. **Is the blur test vacuous?** It asserts `typing()` is true before the click, so the fixture
   cannot silently stop focusing the box.
5. **Does it add cost to the shipped binary?** One public function in `editor.rs`, no calls from
   product code.

## Test diffs

New file `crates/bevy_emerge_mapper/tests/a_palette_click_arms_the_piece.rs` — 3 tests. No existing
test was edited, loosened or skipped.

## Release

No release: E-006 open.

## Next focus

`CARGO_TARGET_DIR=/Users/ladvien/target/emerge-loop` is the warm dir and must be used explicitly
(the repo's own `target/` is not it); starting a second one costs a 10–15 minute Bevy rebuild.
Then E-051 (the `Project` clone per kit click) or the outstanding workspace gate for `203fbc3`.
