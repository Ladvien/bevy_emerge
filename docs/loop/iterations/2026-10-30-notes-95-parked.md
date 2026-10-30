# 2026-10-30 notes: #95 walked end to end; a note that removed a piece cannot be undone (fix parked)

Focus: notes (#95) · Commits: ee6c182 (merge, main, unpushed); 6ce6a6c (local branch `w4/95-notes-end-to-end`, not built) · Machine: laptop · Outcome: nothing shipped

## Summary
Claimed #95 (the laptop has Sanctuary and a display). Walked its flow on a scratch copy of Sanctuary's market in the editor: three notes by the person's route (Shift+N and a drag on empty ground; Shift-click `kitchen_table`; Shift-click `stall_4`; Enter sends all), two agents holding notes at once, the fence refusing `placement_remove stall_4` until `note_ask` and `note_answer`. Keeping worked; `note_undo_work` on the removal note was refused, so the flow's "undo one" fails on the note that asked.

## Baseline
Gate: not run (see below). `accept.py`: not run. Load 33 at start, 66–83 during the pass (other clones' gates and Sanctuary release builds); free disk 28 GB.

## Bugs found
### note_undo_work refuses a note whose work removed a piece
Must-fix: blocks yes (#93's "undo the whole note in one step" is false for removals), common yes (removal is the ask-first case #92 fences), no workaround no (Cmd+Z by hand). Evidence: on the scratch town, `note_undo_work {"id":"n3"}` answered `0040-3315 (placement_remove) changed more than pieces, so note n3's work cannot be undone in one step. Undo it by hand.`
Root cause: `ops_notes::NoteUndoWork` (and `footprint`) read only an inverse's `delta`; `placement_remove` and `stamp_remove` log `put_back`, which is the same row change.
Draft fix (6ce6a6c, branch only): `ops::RowDelta::undoing` reads `delta` or either removal's `put_back`; both callers use it. Regression check drafted: `notes_end_to_end::two_notes_kept_and_one_undone_leave_the_map_file_with_only_the_kept_work`. Not compiled: `cargo test -p bevy_emerge_mapper --test editor notes_end_to_end::` timed out at 1800 s under the load, so "fails before" is not yet observed.

## Enhancement
None. The clip owes one more thing: `note_resolve` needs the operation ids the agent ran, which a storyboard cannot know in advance (they carry the editor's pid); the draft storyboard `tools/demo/storyboards/notes_end_to_end.json` omits them, so as written its undo would revert nothing.

## Review
Not spawned: nothing is proposed for main. Self-review of the draft: (1) does `undoing` change any op that already logged `delta`? no, that path is first and unchanged; (2) is a removal's put-back via `RowDelta::apply` the same as `put_back`? same rows at the same indices ascending, and `apply` refuses an id already on the map, as `restorable` does; (3) footprint now gains a removed piece's cells, so a later op on them is named as a conflict (intended); (4) protected files: `tests/all.rs` gains one `mod` line (owner exception, new editor test file); (5) nothing of it is on main.

## Test diffs
None on main. On the branch: a new test file and its `mod notes_end_to_end;` line in `all.rs`.

## Release
None: not `bevy_emerge_loop`.

## Next focus
Build branch `w4/95-notes-end-to-end`: watch the test fail on main, take the fix, gate it, push; then let the storyboard read op ids (or resolve take them from the log) and record #95's clip.
