# 2026-10-30 relations: a relation's end is its whole stack for ClearInFront and VisibleFrom (#188)

Focus: relations (bug issue #188, own claim) · Commits: 7a29aee · Machine: bmb · Outcome: fixes only

## Summary
`ClearInFront` counted what rests on the subject (a jug on the counter) as an intruder, and `VisibleFrom` counted the host an end rests on (the stall a sign hangs on) as a blocker, so both read broken on any dressed map and `relations_solve` could not mend them. Both now treat each end's whole stack as that end, by the rule `walk` already used for `ReachableFrom`. The fix and its test were my pass 14's parked tree (`wiggum/parked-pass-14-20261030-151140`, ba573c0), adopted, built, tested and gated this pass.

## Baseline
Gate: green at eaa022e (main). `accept.py`: not run (core-only change to two relation checks; running it beside the 35-45 min push gate on bmb would have pushed the pass past budget). Load 4.7, free disk 22 GB, RAM 48 GB.

## Bugs fixed
### #188: Relations: a piece's own stack counts against it
Must-fix: blocks y (the `RELATIONS` row and `emerge_state.relations` report a permanent break that `relations_solve` cannot mend), common y (any dressed map), no workaround y. Evidence: issue inputs (a) and (b).
Root cause: `check_one`'s `ClearInFront` arm excluded only the subject's own id; `blockers` excluded what rests on an end but not what an end rests on. · Regression check: `crates/bevy_emerge_core/tests/a_relations_end_is_its_whole_stack.rs`, 6 tests, all failed at eaa022e (`the_host_an_end_rests_on_does_not_block_the_sight_line`: `left: [Unmet { measured: 0.0, by: Some("stall") }] right: [Met { measured: 1.0 }]`), all pass after. `cargo test -p bevy_emerge_core`: 0 failures; clippy `-D warnings` clean; full workspace gate on push.

## Enhancement: none
The pass's item was the bug issue.

## Borrowed from Mixar
none

## Performance
not measured: no per-frame path changed (the check runs once a second on the `RELATIONS` row and on demand).

## Acceptance
not run, see Baseline.

## Review
Reviewer `Review188` ran over 7a29aee: no blocking finding. Correct for both kinds at any stack depth and for a subject that itself rests on a host; `solve` can no longer propose moving an end's own stack (a moved stranger's bottom can never be an end's bottom). Accepted consequence, the same as `ReachableFrom`'s since f438904: a stranger on the subject's own host (a stool on the platform a counter stands on) counts as part of the subject. Non-blocking: `docs/guide/agents.md:115` was stale; fixed in the docs commit.

## Test diffs
One new test file, `crates/bevy_emerge_core/tests/a_relations_end_is_its_whole_stack.rs`; no existing test changed.

## Release
None: not `bevy_emerge_loop`.

## Next focus
The next unclaimed issue in focus order, or a headless backlog item (E-046 first).
