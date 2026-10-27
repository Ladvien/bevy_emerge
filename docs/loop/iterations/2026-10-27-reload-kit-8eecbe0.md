# 2026-10-27 reload-kit: a kit's save re-reads that kit, not the whole project (E-003)

Focus: editor file watcher · Commits: `dbdac3a..8eecbe0` · Machine: laptop (`Cs-MacBook-Pro.local`) ·
Outcome: shipped

## Summary

`watch::adopt` answered a save to a kit's `library.ron` or `project.ron` by calling `Project::open`
again: `kits.ron`, the vocabulary, every bound kit's library and policy, and a triangle count for
every `.glb` in the merge — on the file an author edits most, per save. `Project::reload_kit` now
re-reads one directory, re-merges from the layers the project already holds, and re-does every
check the open makes over the merge before swapping, so a refusal still leaves the last good
project whole. The project-wide files (`kits.ron`, `vocab.ron`, `compositions.ron`, `rules.ron`)
still go through a whole open, because nothing smaller knows how to read them.

This pass also finished and repaired a **killed pass's uncommitted E-003 work**: it compiled but
two of its three tests were red, and one of them was measuring nothing (below).

No enhancement beyond E-003 was started; the 45 minutes went on two cold builds and the
measurement repair.

## Baseline

Gate before, at `0f0b4c8`: tree **dirty** with the killed pass's E-003 diff (3 modified, 2 new files).
`backlog.py check` consistent. `cargo check -p bevy_emerge_mapper --all-targets` exit 0 — the diff
compiled, which is why the dirt was finished rather than reverted.

`accept.py --source assets`: **10/10**, first failure none (after the change; see Acceptance).
**No `before` run**: the baseline is the last recorded run on this machine and this source (also
10/10, `25ac219`'s log line).

Load average **15.8–18.8** throughout on 10 cores: two unrelated `cargo build --release --examples`
jobs in `/Users/ladvien/bevy_elements` (pids 61705, 71855) held the machine for the whole pass, and
I did not kill them. One test build exceeded a 1500 s deadline; the same build then took 14 m 53 s.
Disk 107 GB free.

Gate after (`8eecbe0`): clippy `--workspace --all-targets -- -D warnings` **clean**;
`a_kit_save_rereads_only_that_kit` 3/3; `backlog.py check` consistent. The workspace suite is
recorded in the LOG line with its own numbers rather than asserted here.

## Bugs fixed

### The killed pass's own measurement measured nothing

`a_kit_save_relayers_only_that_kit` failed with `re-layered []` — zero, not two. The counter was a
`thread_local`, and Bevy 0.20's `default_executor` is `MultiThreadedExecutor`
(`bevy_ecs-0.20.0/src/schedule/executor/mod.rs:48`), so `watch::adopt_changes` does the layering on
a task-pool thread while the test reads the counter on the libtest thread. **The test would have
passed against the unfixed code for the wrong reason** had `adopt` ever run on the caller's thread.

Root cause: a `thread_local` counter is right only when the work runs on the caller's thread —
which is why `tiles::library_writes_for_test` (an in-editor commit) can be one and this cannot.
Fix: `policy::layered_for_test` and `project::triangles_measured_for_test` are now
`std::sync::Mutex<Vec<_>>`, and the test file holds a file-level lock for the whole body of each
test so no sibling test's layering lands inside its window.

Regression check: `a_kit_save_rereads_only_that_kit` — failed before (`left: 0, right: 1`), passes
after.

### The same test asserted against the wrong directories

`the_edited_piece_is_drawn_at_its_new_height` panicked "the map's piece was never spawned": the
fixture's `place("lamp", ..)` mints the row id `lamp@0`, so the wall it meant to measure was
`site/wall@1`. And the triangle assertion compared `.glb` paths against the *kit directory*, but a
kit's meshes live in a *pack* directory (`assets/beta/wall.glb` for descriptor `site/wall`), so it
would have failed on any project once the counter worked.

Fix: the row id is `site/wall@1`; the expected `.glb` set is derived from the project's own kits
(every kit but the edited one), with a guard that the set is non-empty so the test cannot pass by
comparing nothing.

Test diffs: both are in a **new** test file this pass authored, so nothing pre-existing was
loosened. The assertion is stricter than what it replaced (it names the other kits' meshes rather
than testing a path prefix).

## Enhancement: E-003, a kit's save re-reads that kit

Goal: a save to one kit's two files must not cost a whole-project open.

Files: `crates/bevy_emerge_mapper/src/project.rs` (`Project::reload_kit`, `triangles_of`, the
triangle counter), `crates/bevy_emerge_mapper/src/watch.rs` (`adopt` routes a kit's file to it),
`crates/bevy_emerge_core/src/policy.rs` (the layering counter),
`crates/bevy_emerge_mapper/examples/open_cost.rs` (the measurement behind the figures),
`crates/bevy_emerge_mapper/tests/a_kit_save_rereads_only_that_kit.rs`.

`reload_kit` re-does, against the new merge and in the same order as `Project::open`: the
namespace the binding declares, `Library::validate` (duplicate ids), `labels::settle_library`
(implied effects), `validate_lattices`, `Library::resolve` (masks), `composition::validate` and
`rules::validate_against`. Nothing touches `self` before all of them pass. `vocab.check_slots` is
deliberately not re-run: it takes no library and neither `vocab.ron` nor `compositions.ron` can be
changed by a kit's own two files.

Proof: `a_kit_save_relayers_only_that_kit` fails at the base with `left: 2, right: 1` (both kits
re-layered) and passes after (1). Proved by making `adopt` skip the per-kit branch
(`kit_dir.filter(|_| false)`), which is the same code path `Project::open` takes:

```
a save to one kit's library re-layered [.. "/assets/emerge/furniture", ".. /assets/emerge/site"]
  left: 2   right: 1
```

MCP: no operation, tool or undo path changed — this is the file watcher's own door, and the
operation surface is untouched.

## Borrowed from Mixar

none.

## Performance

`perf.py` not run: this pass is on the laptop and the machine was at load ~16 with two foreign
release builds for its whole duration, so a timing would measure them.

The cost is instead pinned deterministically, as work rather than time (E-011's lesson):
**1 kit layered instead of 2, and no other kit's `.glb` opened for a triangle count.** The figures
the change is aimed at are in `reload_kit`'s doc comment and re-takeable in one command:
`cargo run -p bevy_emerge_mapper --example open_cost -- <project-root>`.

## Acceptance

`reached 10/10` on `--source assets`, first failure none. Equal to the last recorded run on this
machine and source, not a before/after pair (no before-run, reason above).

## Review

Reviewer could not run in this pass's window — the build queue under load ~17 meant a subagent's
diff review would have been the only thing moving while a 15-minute compile ran; this is the
self-review fallback, and these answers are mine:

1. **Does the cheap path give a different answer from the open?** No, and that is asserted rather
   than argued: the same validation functions, in the same order, before the swap; and
   `a_kit_edit_that_breaks_a_composition_is_refused..` proves a project-wide refusal still fires
   and leaves the old project.
2. **Can a kit's own file change something outside that kit?** No: the binding is `kits.ron`, and
   the vocab, compositions and rules are project files, all still on the whole-open path. A
   re-skin pair is two directories providing one namespace and only one is bound, so the other is
   not `project.kits` at all.
3. **Is the measurement sound?** It was not, and the first fix made it process-wide; the file-level
   lock is what keeps a process-wide count meaningful. The negative control (forcing the open path)
   fails with 2 vs 1, so the counter sees the work.
4. **Any allocation or copying added to compiled hot paths?** `reload_kit` clones the untouched
   kits' descriptors to build the merge — the same cost `Project::open` already pays for those rows,
   once per save, and less work overall. `triangles_of` adds one mutex push per `.glb`.
5. **Would this pass's fix survive a later refactor of `adopt`?** The test asserts the *work*
   (directories layered, files opened), not which function was called, so moving the branch keeps
   the test honest.

## Test diffs

New file `crates/bevy_emerge_mapper/tests/a_kit_save_rereads_only_that_kit.rs` (3 tests) — added by
this pass (finishing the killed pass's work), not loosened: it was red when found, and every
assertion it carries now is stricter than the version that failed.

## Release

No release: E-006 open.

## Next focus

E-086 still blocks any new operation: `UI_ENTRY_POINTS`/`CHOOSER_ENTRY_POINTS` live inside the
protected parity test, so `map_remove` (E-066) cannot be added by a loop pass. Awaiting an evaluator
pass to move both tables into `ops::ui_entry_points()`.
