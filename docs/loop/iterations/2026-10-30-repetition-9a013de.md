# 2026-10-30 repetition: a round piece repeats at any turn (#184)

Focus: repetition (lens #5) · Commits: 9a013de · Machine: big · Outcome: shipped

## Summary
`repeats` and `vary` took a turn as the cheapest variation, so round barrels turned 0/90/180° read as no repeat and `repetition_vary` reported `left: 0` while a player still sees three stamped barrels. `repetition.ron` now carries `round`, the `kind` tokens of pieces that look the same at any yaw: their copies repeat whatever their yaw, and `vary` never turns one but counts it in `left`. Empty by default, so no project's report changes until its author says a kind is round.

## Baseline
Gate: clean tree at 18fe327, core repetition tests green. `accept.py`: not run this pass (load 48 on 24 cores, 7 GB available; the last big run was 10/10 and this change is a core rule whose default is empty, so the fixtures' reports are unchanged). Load 47.9, free disk 307 GB, available RAM 7 GB.

## Bugs fixed
None. Filed bugs: #182 w4, #180 w8, #179 w2 claimed; #152 half 2 and #149/#150 need protected edits (owner).

## Enhancement: round kinds in repetition.ron (#184)
Goal: a repeat that turning cannot end is reported, not hidden · Files: `crates/bevy_emerge_core/src/repetition.rs`, `docs/guide/data-files.md` · Proof: `crates/bevy_emerge_core/tests/a_round_piece_repeats_at_any_turn.rs` failed to compile at base (`no field 'round' on type RepetitionRules`, and `(round: [...])` is an unknown field under `deny_unknown_fields`), 4/4 after; `vary_turns_clutter_out_of_a_repeat` 5/5 and `repeated_clutter_outranks_repeated_structure` 4/4 unchanged.
MCP: no new operation or tool; `emerge_state.repetition`, the REPEATS line and `repetition_vary` read the new field through the existing `load_repetition`, re-read on every call. Undo and preview of `repetition_vary` are unchanged (a round piece is simply never in its turned rows).
Assumed decision (on #184): options (a) `round` kind tokens in `repetition.ron`, (b) infer from the mesh (`align.front == None` and a square footprint), (c) a per-descriptor field. Picked (a): data the author tunes beside `structure`, no saved descriptor schema change, and (b) would assert a fact about the art (a label, a texture) that a centroid cannot see.

## Borrowed from Mixar
None.

## Performance
Not measured: `perf.py` needs a display (big has none). `vary` does one descriptor lookup per placement as before; the round test is a bool per pair.

## Acceptance
Not run (see Baseline).

## Review
Reviewer ran (one `reviewer` subagent over 9a013de). No P0/P1. P2: `repetition_vary`'s summary blamed every left copy on "more copies than four turns can tell apart", false for round copies; reworded in `ops_repetition.rs` ("..., or round pieces no turn changes"). P3s fixed in the same follow-up commit: `Variation::left`'s doc, the guide's live-reload row ("never a round one"), the `ops_brp.rs` and `ops_repetition.rs` comments, and the MCP `emerge_state` description (main.rs and docs.rs) with `docs/mcp-tools.md` regenerated (`docs_are_current` 6/6). It judged each of the four tests to catch a plausible bug (the turned-barrels case catches a `vary` that skips round pieces but keeps the yaw check).

## Test diffs
One new test file, `crates/bevy_emerge_core/tests/a_round_piece_repeats_at_any_turn.rs` (core tests are one binary per file, so nothing else changed). No existing test touched.

## Release
None: not `bevy_emerge_loop`.

## Next focus
A project-side default: the fixture kits declare no round kinds, so `round` is opt-in until an author lists a kind (an assets edit, not a code change).
