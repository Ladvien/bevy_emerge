# bevy_emerge

Editor, learned grammar and WFC for Bevy games. Workspace of crates under `crates/`; fixtures under `assets/`.

## Version pin

Pinned to **Bevy 0.20.0** (`[workspace.dependencies] bevy`). Read the vendored source and migration guides, not bevy.org (which tracks `main`): `~/.cargo/registry/src/index.crates.io-*/bevy-0.20.0/` with `examples/` and `_release-content/migration-guides/`. `bevy_emerge_anim` and `bevy_devshot` name `0.20.0` themselves because they need `default-features = false`; bump them by hand with the workspace.

## Working with consumers

Games depend on these crates by path and the editor runs on a game as its project. The two evolve together:

- Changes a game needs are made here, in their own commits, with tests here. Never copy a crate into a game or patch it there.
- Every API change lists the consumer callsites it breaks.
- Before pushing, run `cargo test --workspace` here and `cargo test` in each consumer.
- If a consumer change depends on an unpushed commit here, the consumer's agents are told which commit (Sanctuary: `AGENTS_CHAT.md`).

Consumers: Sanctuary (`/Users/ladvien/sanctuary`, decision 0084).

## Rules

- `bevy_emerge_core`, `bevy_emerge_rng` and `bevy_emerge_solver` are engine-free. `bevy_emerge_core/tests/engine_free.rs` fails on any `bevy*` dependency except the exact names in `SIBLING_DEPS`.
- Licence is MIT OR Apache-2.0 for every crate. Do not copy GPL code in from foundation_vs_slop's untouched crates.
- One plugin per domain; split by domain, not by kind.
- Do not hard-wrap markdown.
- Close any Bevy app you launch for a test.
