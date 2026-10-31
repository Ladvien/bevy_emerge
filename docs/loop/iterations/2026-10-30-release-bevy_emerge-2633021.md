# 2026-10-30 release: bevy_emerge 0.1.0 is on crates.io; anim's verify build is next

Focus: E-006 (#128) · Commits: docs only on main · Machine: laptop · Outcome: shipped (release; no product change)

## Summary
The verify build that pass 151 started at 18:09 (`cargo publish -p bevy_emerge --dry-run -j 4`, pid 74854) finished at 20:09 after 119 min 56 s, and `/tmp/loop151_release.sh` went on by itself: the real publish verified again in 2 min 9 s (cached), `bevy_emerge` 0.1.0 was uploaded, and `bevy_emerge-v0.1.0` was tagged and pushed. The crates.io index lists it (`https://index.crates.io/be/vy/bevy_emerge`, created 2026-10-31T01:11Z, deps `bevy ^0.20.0` and `bevy_emerge_core ^0.1.0`). The script then started `bevy_emerge_anim`'s dry-run at 20:11. README and `docs/guide/your-game.md` now say `bevy_emerge` is on the registry.

## What I checked before leaving it alone
- Packaging for both pending crates: `cargo package -p <crate> --list` (own `CARGO_TARGET_DIR`, no lock) lists every source file, README, examples and tests; no `include_str!`, `include_bytes!` or `CARGO_MANIFEST_DIR` path leaves either crate. So the hours-long verify cannot fail on a missing file.
- Why anim's verify is not cheap: `bevy_emerge_anim` asks for `bevy` with `default-features = false` and four features, a different feature set from `bevy_emerge`'s, so its verify recompiles Bevy (its first units were `syn` and `hashbrown`; its tree is 318 crates against 535). 16 new `.rmeta` in the first 6 minutes, about 2.7 a minute, so roughly 100 minutes if the laptop stays as it is.
- `cargo info bevy_emerge@0.1.0` inside the clone answers from the workspace member, not the registry. Use the sparse index (`curl https://index.crates.io/be/vy/bevy_emerge`) to prove a publish from here.

## Baseline
Gate: not run (docs only; fast path). `accept.py`: not run. Load 24, free disk 25 GB, 5 GB unused RAM, swap 5.84 of 6.14 GB (#195). Five Bevy builds share the laptop: this clone's verify, and the bevy_elements, bevy_carnage, bevy_speech_bubbles and isomesh loops.

## Why no code this pass
The verify build holds the clone's `target/emerge-loop/debug/.cargo-lock`, so the push gate's `cargo test --workspace` and slice 3's tests (`loop/129-slice3-rebased-153`) would wait for it again. Slice 3 stays unbuilt; hosts with room: bmb has 1.3 GB free disk and 15 of 15 GB swap, big is at load 42 with 1 GB available, sib does not answer on ssh, and the Home Cloud boxes (`one` to `four`, 4 cores, idle) have no C compiler or pkg-config, so a Bevy build there needs a system package install nobody has approved.

## Test diffs
None.

## Release
Published: `bevy_emerge` 0.1.0 (https://crates.io/crates/bevy_emerge/0.1.0), tag `bevy_emerge-v0.1.0` on origin. Earlier: `bevy_emerge_rng`, `bevy_emerge_solver`, `bevy_emerge_core`, `bevy_emerge_snip` 0.1.0. Left: `bevy_emerge_anim` 0.1.0 (the script publishes, tags and pushes it when its verify ends).

## Next focus
When `/tmp/loop151_release.log` shows `=== DONE`, the lock is free: cherry-pick `31f8ef8 1876e02` (#129 slice 3), run `operations_have_two_front_ends` and `placement_set_tip` in the editor binary, push with `LOOP_GATE_CARGO=$HOME/.local/share/wiggum/gate-cargo-t4.sh`; close #128 with the crates.io URLs.
