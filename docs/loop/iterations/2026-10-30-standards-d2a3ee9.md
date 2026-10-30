# 2026-10-30 standards: piece_import names the standards an imported piece breaks

Focus: kit standards (#138 / E-089) · Commits: d2a3ee9 · Machine: big · Outcome: shipped

## Summary
An agent importing a `.glb` now learns at once which of the project's `standards.ron` rules the new piece breaks: `piece_import`'s applied answer carries `after.standards {breaches, error}` for that piece alone, and the summary names the broken rules. Reported, not refused.

## Baseline
Gate: clippy `-D warnings` on `bevy_emerge_mapper` clean; push hook runs `cargo test --workspace`. `accept.py`: not run (load ~21 on big, shared with other workers). Load 21, free disk 422 GB, available RAM 13 GB.

## Bugs fixed
None.

## Enhancement: #138 slice 3
Goal: breaches reported on admission · Files: `ops.rs` (piece_import commit), `ops_brp.rs` (`piece_standards_json`, shared `load_standards`/`breach_json`) · Proof: `tests/an_imported_piece_names_the_standards_it_breaks.rs` failed before (`after.standards` null, 2 failed), 2 pass after; `emerge_state_reports_standards_breaches` and `an_agent_can_import_a_mesh` still pass (11/11).
MCP: no new operation; `piece_import`'s answer gains a field. Undo unchanged. Preview unchanged (the plan's descriptor is pre-policy, so breaches are reported only after commit, against the policy-applied library).

## Borrowed from Mixar
None.

## Performance
Not measured: no per-frame path touched; `perf.py` cannot run on big.

## Acceptance
Not run (load).

## Review
Reviewer not spawned (budget). Self-review: (1) correctness: breaches computed on the policy-applied descriptor, the same one `emerge_state` checks; (2) a bad file never refuses the import, it is named in `error`; (3) only the imported piece is checked; (4) slice 2's error string is unchanged (`<path>: <parse error>`); (5) no protected test changed beyond the granted `all.rs` mod line.

## Test diffs
New file `an_imported_piece_names_the_standards_it_breaks.rs`; one `mod` line in `tests/all.rs` (owner exception, new editor test file).

## Release
None: not `bevy_emerge_loop`.

## Next focus
#138's last slice: breaches in the Meshes tab (layout assertion headless; clip owed).
