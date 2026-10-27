# 2026-10-27 core: Extent::height documented as base-to-top, checked against the glb

Focus: core · Machine: laptop · Outcome: shipped (issue #83 criteria 1 doc, 2, 3; #82 already fixed by 089705f)

## Summary
`Extent::height` doc names the base as reference; `CHANGELOG.md` announces the 089705f behaviour change with the migration; `Library::height_disagreements(asset_root, tolerance_m)` returns disagreements and an `unmeasured` list.

## Proof
`cargo test -p bevy_emerge_core --test a_declared_height_is_checked_against_the_mesh` (new; fixture glb, one honest, one 3.7 cm short → error -0.037, non-surface skipped, unreadable listed). Core suite green, clippy -D warnings clean. `accept.py` not run: no behaviour in the accepted paths changed.

## Not done
#83 criterion 1's assertion in `a_guest_lands_on_its_hosts_lid.rs` (protected test file); criteria 4 and 5 (needs Sanctuary's library; mapper warning optional).

## Release
No release: E-006 open.
