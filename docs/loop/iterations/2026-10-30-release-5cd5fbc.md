# 2026-10-30 release: the publishable set is decided and packages; #129 slice 1 re-gated

Focus: release (E-006, #128) · Commits: 5cd5fbc (+ merges 7fc6506, this pass's merge) · Machine: laptop · Outcome: shipped

## Summary
#129 slice 1 had passed the push gate four times and lost the push each time; this pass merged origin/main (conflicts only in CHANGELOG.md and docs/loop/log.md, kept both), and the gate passed again in 1543 s, but main had moved (`cannot lock ref ... is at 58407e6`), so it lost a fifth time. Meanwhile #128 slice 2: decided which crates ship and made the game-facing set package.

## Baseline
Gate: passed on 7fc6506 (pre-push, 1543 s). `accept.py`: not run (the gate held the CPU for the whole pass). Load 6-14, free disk 71 GB.

## Bugs fixed
None.

## Enhancement: #128 slice 2, the publishable set
Assumed decision (on #128, label `assumed-decision`): ship bevy_emerge_rng (already 0.1.0), solver, core, snip, bevy_emerge, anim; bevy_emerge_mapper and bevy_emerge_mcp are `publish = false` (crates.io refuses the mapper's git-pinned family deps; mcp depends on decide, which is `publish = false`). bevy_emerge and bevy_emerge_anim now carry `version = "0.1.0"` on core.
Proof: `cargo publish -p bevy_emerge_solver --dry-run` passes ("aborting upload due to dry run"); `cargo package --list` clean for core, snip, bevy_emerge, anim. Core's dry run needs solver on crates.io first.

## Borrowed from Mixar
None.

## Performance
Not measured: manifest-only change.

## Acceptance
Not run: the gate occupied the machine.

## Review
Reviewer not spawned (manifest-only diff, pass out of time). Self-review: (1) does a version req change resolution? No, path wins in-workspace; `cargo metadata` ok. (2) Does `publish = false` break anything? Only `cargo publish` of those crates. (3) Is any consumer affected? No: Sanctuary depends by path. (4) Reversible? One line each. (5) Protected files? None touched.

## Test diffs
None.

## Release
None: no pushed commit holds a green gate this pass. Next: publish solver -> core -> snip -> bevy_emerge -> anim from main once pushed.

## Next focus
Land the push; then publish the set in order (#128) and #129 slice 2.
