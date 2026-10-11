# bevy_emerge_core

> ⚠️ **Vibe Coded** — written by an AI agent working from a human's direction. It is used in a shipping game and covered by tests, but it has had no line-by-line human audit. Read it before you trust it.

The engine-free half of world building: the asset descriptor and map schemas, a constraint IR with three solver backends, WFC, Poisson/Delaunay geometry, a seeded ChaCha8 RNG, comment-preserving RON surgery, a hand-rolled glTF reader, and animation-clip measurement.

> **Part of [`Ladvien/bevy_emerge`](https://github.com/Ladvien/bevy_emerge)** (`crates/bevy_emerge_core/`), MIT OR Apache-2.0, Bevy 0.20. It began as `crates/emerge-core/` in [`Ladvien/foundation_vs_slop`](https://github.com/Ladvien/foundation_vs_slop) with the history kept; that repo still carries the GPL Bevy 0.19 original until it ports to this one. Issues and PRs belong in `bevy_emerge`.

## Why "engine-free" is enforced, not just intended

Three consumers share this crate — a game, an offline search that fans out across worker subprocesses, and a standalone editor — and none of them should have to agree on a renderer to share a schema.

That was a *comment* for months, and a comment cannot fail a build. `tests/engine_free.rs` is the ratchet: it fails if a `bevy`/`avian`/`wgpu`/`winit` dependency appears in the manifest, and a second test scans the sources as a backstop. Widening the dependency list should cost an argument and a deliberate edit, not a passing `cargo build`.

## Examples

All three print to the terminal — no engine, no assets, no GPU.

```sh
cargo run -p bevy_emerge_core --example wfc_grid  # WFC over a terrain alphabet you define
cargo run -p bevy_emerge_core --example poisson   # Poisson-disk sites → Delaunay → degree cap
cargo run -p bevy_emerge_core --example bevy_emerge_rng   # the one seeded generator everything draws through
```

`wfc_grid` gives `collapse_grid` a five-step terrain ramp with one adjacency rule and renders the result as ASCII, including a run with the top and bottom rows pinned. `poisson` reports the closest pair and the degree distribution before and after pruning. `bevy_emerge_rng` shows two streams from one seed matching exactly, and `below(n)` staying unbiased over 700k draws where a modulo reduction would skew.

## License

MIT OR Apache-2.0, see the repository root's `LICENSE-MIT` and `LICENSE-APACHE`.
