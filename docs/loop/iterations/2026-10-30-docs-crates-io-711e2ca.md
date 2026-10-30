# 2026-10-30 docs: README says which crates are on crates.io; the release waits on a starved laptop

Focus: E-006 (#128), #129 · Commits: b0d8cba..711e2ca · Machine: laptop (mac_air) · Outcome: docs only; the two remaining publishes still running

## Summary
No code was built: the pass found `/tmp/loop151_release.sh` still in its first `cargo publish -p bevy_emerge --dry-run` (started 18:09, verify build compiling Bevy into the shared `target/emerge-loop/debug`, which holds cargo's build lock against any other build of this clone), on a host with swap 5.9 of 6.1 GB full and 12-16 `rustc` at under 1 % CPU (#195). README's Status said no crate could be published; four are on crates.io, so it now says which, and says `bevy_emerge` and `bevy_emerge_anim` are next. Nothing else this clone can build finishes inside a pass until the release ends.

## Baseline
Gate: not run (docs only; pre-push took the docs fast path, backlog and changelog checks passed in 44 s). `accept.py`: not run (no product code). Load 18-19, free disk 32 → 27 GB (the verify build), PhysMem 29 of 32 GB used, compressor 7.7 GB. OTEL unreachable (`No route to host`); by ssh: big load 42, 5 GB available, swap 26 of 31 GB; bmb swap 11.8 of 12.3 GB; sib timed out.

## Bugs fixed
None. Open bugs read this pass: #193 (claimed, w8), #194 (duplicate of #193), #152 half 2 and #149/#150's hook step (protected, the owner's), #195 (assumed decision, the owner's).

## Enhancement
README "Status", "Use it in your game" and `docs/guide/your-game.md` step 1 now say what `curl https://crates.io/api/v1/crates/<name>` returned at 18:20: `bevy_emerge_rng`, `_solver`, `_core` and `_snip` at 0.1.0; `bevy_emerge` and `_anim` do not exist yet. The reason the editor, mcp, decide and devshot stay `publish = false` is stated (git-pinned family crates; crates.io refuses them).

## Release
Owed, unchanged: `bevy_emerge` and `bevy_emerge_anim` (log `/tmp/loop151_release.log`; the script publishes, checks `cargo search`, tags HEAD and pushes the tag, serially). The pass after it ends adds the changelog line with `--no-clip "a registry release; proved by cargo info <crate>@0.1.0"`, sets E-006 `fixed`, edits the README sentence to list all six, and closes #128 with the five URLs.

## Test diffs
None.

## Next focus
Read `/tmp/loop151_release.log`. If the release has ended, finish E-006 as above, then build #129 slice 3 from `loop/129-slice3-tip-149` (cd1265c; keys.rs, ops.rs, three test files; unbuilt).
