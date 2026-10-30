# 2026-10-30 repetition: a project's repetition.ron decides what is structure

Focus: repetition (#139, research lens #5) · Commits: e8f9edf · Machine: big (bevy_emerge_w3) · Outcome: shipped

## Summary
`RepetitionRules` was serde data an author should tune, but `emerge_state.repetition` and the REPEATS status line both used the built-in defaults. `<project_dir>/repetition.ron` now decides (absent = defaults, partial = defaults for the rest, bad = error naming the file), both readers share `ops_brp::load_repetition`, and `watch.rs` watches the file so the REPEATS line follows an edit.

## Baseline
Gate: clippy `-D warnings` clean; push hook runs `cargo test --workspace`. `accept.py`: not run (load ~30 on big). Load 30, free disk 418 GB, available RAM 15 GB.

## Bugs fixed
None.

## Enhancement: project repetition rules
Goal: a project names its own structure kinds and repeat radius, live. · Files: `bevy_emerge_core/src/repetition.rs` (`FILE`, `RepetitionRules::parse`), `bevy_emerge_mapper/src/ops_brp.rs`, `editor.rs` (REPEATS), `watch.rs`, MCP tool text, `docs/guide/data-files.md`, `docs/mcp-tools.md` (regenerated). · Proof: `tests/a_projects_repetition_rules_decide_what_is_structure.rs` failed before (3 of 4: role stayed `clutter`, no `error`, edit ignored), 4 pass after; `emerge_state_names_visible_repeats` and watch suites 11/11; `docs_are_current` 6/6.
MCP: no new tool or operation; `emerge_state.repetition` gains `error`. Read-only, no undo needed.

## Borrowed from Mixar
none

## Performance
not measured: perf.py needs a display. The REPEATS line reads a small file only when the map or project changed (existing guard).

## Acceptance
not run (load ~30).

## Review
Reviewer not spawned (model credits exhausted last pass, 402). Self-review: (1) bad file falls back silently? No: error in state, `repetition.ron unreadable` in ACCENT on the line. (2) Every-frame IO? No, guarded by `open/project.is_changed()`. (3) Watcher adoption of the file? Returns parse result; mut borrow marks project changed. (4) Schema version? None added; the file is a partial serde struct with `deny_unknown_fields`. (5) Existing tests changed? Only `all.rs` gains its `mod` line (owner exception).

## Test diffs
New `a_projects_repetition_rules_decide_what_is_structure.rs`; `all.rs` +1 `mod` line (owner exception, new editor test file).

## Release
None: not bevy_emerge_loop.

## Next focus
Clip for E-090 owed (no display on big); next headless backlog/research item.
