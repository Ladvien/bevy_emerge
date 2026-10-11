# bevy_emerge

> ⚠️ **Vibe Coded** — written by an AI agent working from a human's direction. It is used in a shipping game and covered by tests, but it has had no line-by-line human audit. Read it before you trust it.

The runtime half of emerge: take an `bevy_emerge_core` library plus a map, and put it in a Bevy world.

> **Part of [`Ladvien/bevy_emerge`](https://github.com/Ladvien/bevy_emerge)** (`crates/bevy_emerge/`), MIT OR Apache-2.0, Bevy 0.20. It began as `crates/bevy_emerge/` in [`Ladvien/foundation_vs_slop`](https://github.com/Ladvien/foundation_vs_slop) with the history kept; that repo still carries the GPL Bevy 0.19 original until it ports to this one. Issues and PRs belong in `bevy_emerge`.

## What it does

Resolves masks, stacked Y, role masks and seat positions once at load, then spawns entities. `spawn_descriptor` is deliberately the **single** shared spawner, so a map cannot look one way in the editor and another in the game — the fork that happens the moment two spawners exist.

This is the only crate in the emerge set that knows what a renderer is, on purpose.

## Examples

```sh
cargo run -p bevy_emerge --example spawn_headless
```

Builds a library and a map in code, shows a map naming an undefined descriptor being refused at load, then hands the valid one to `EmergePlugin` and counts the spawned entities. Runs with `WgpuSettings { backends: None }`, so it needs no GPU and no asset on disk — what is being demonstrated is placement, not pixels.

## License

GPL-3.0-only, with the game it was carved out of.
