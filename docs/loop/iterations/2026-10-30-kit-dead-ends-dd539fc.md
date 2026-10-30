# 2026-10-30 kit stress: a kit names every edge face no piece of it can meet

Focus: research lens #4 (kit stress tests) · Commits: dd539fc · Machine: big · Outcome: shipped (slice 1 of #142)

## Summary
No own claim; every open issue was claimed by another worker or needs `clef-flash`/a display/Sanctuary, and the focus's headless backlog list is closed except B-021 (unreproduced). Filed and claimed #142 from lens #4: `adjacency::dead_ends(library, divisions)` tries every piece at every quarter turn and near/centre/far alignment against each declared face, through `faults()` itself, and names the faces nothing meets.

## Baseline
Gate: clean tree at da687b2. `accept.py`: not run (load 30–49 on big). Load 29.8, free disk 405 GB, available RAM 12 GB.

## Bugs fixed
None.

## Enhancement: kit dead-end faces
Goal: find a socket no piece presents back without placing pieces. · Files: `crates/bevy_emerge_core/src/adjacency.rs` · Proof: `tests/a_kit_names_every_face_nothing_can_meet.rs` (4 pass; failed before as unresolved `dead_ends`). Both outcomes are exercised: an asymmetric jamb face is named, its partner clears it, a uniform face is met by its own piece turned.
Smoke: `assets/emerge/site` and `site_greybox` (45 pieces each) report 0 dead ends in ≤ 1.5 ms. MCP: none yet (slice 2 surfaces it).

## Borrowed from Mixar
None.

## Performance
Not measured with `perf.py` (no display on big); nothing per-frame changed.

## Acceptance
Not run (load).

## Review
Reviewer not spawned (model credits 402 in the last passes). Self-review: (1) one path — a face counts as met only when `faults()` on the trial map is empty and the seam carries a declared token; (2) lenient by design, so a reported dead end is certain; (3) pieces without footprint/height/lattice skipped as `faults` skips them; (4) no RNG, no write; (5) a 1-wide lattice shares cells between its E and W faces, so it always meets itself — that is `faults`' rule, not this check's.

## Test diffs
New file `crates/bevy_emerge_core/tests/a_kit_names_every_face_nothing_can_meet.rs`; no existing test changed.

## Release
None: not `bevy_emerge_loop`.

## Next focus
#142 slice 2: show dead ends in `emerge_state` / the Meshes pane.
