# bevy_grimoire

> ⚠️ **Vibe Coded** — written by an AI agent working from a human's direction. It is used in a shipping game and covered by tests, but it has had no line-by-line human audit. Read it before you trust it.

The runtime half of grimoire: take an `bevy_grimoire_core` library plus a map, and put it in a Bevy world.

> **Part of [`bevy-coven/bevy_grimoire`](https://github.com/bevy-coven/bevy_grimoire)** (`crates/bevy_grimoire/`), MIT OR Apache-2.0, Bevy 0.20. It began as `crates/emerge-bevy/` in [`Ladvien/foundation_vs_slop`](https://github.com/Ladvien/foundation_vs_slop) with the history kept; that repo still carries the GPL Bevy 0.19 original until it ports to this one. Issues and PRs belong in `bevy_grimoire`.

## What it does

Resolves masks, stacked Y, role masks and seat positions once at load, then spawns entities. `spawn_descriptor` is deliberately the **single** shared spawner, so a map cannot look one way in the editor and another in the game — the fork that happens the moment two spawners exist.

This is the only crate in the grimoire set that knows what a renderer is, on purpose.

## Examples

```sh
cargo run -p bevy_grimoire --example spawn_headless
```

Builds a library and a map in code, shows a map naming an undefined descriptor being refused at load, then hands the valid one to `GrimoirePlugin` and counts the spawned entities. Runs with `WgpuSettings { backends: None }`, so it needs no GPU and no asset on disk — what is being demonstrated is placement, not pixels.

## License

MIT OR Apache-2.0, see the repository root's `LICENSE-MIT` and `LICENSE-APACHE`.
