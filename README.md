# bevy_emerge

[![CI](https://github.com/Ladvien/bevy_emerge/actions/workflows/ci.yml/badge.svg)](https://github.com/Ladvien/bevy_emerge/actions/workflows/ci.yml)

bevy_emerge learns a world's grammar from an example map that a person authors, then assembles new layouts from it with wave function collapse. It is for Bevy 0.20.

![A person snips a box over a wall and writes a note, an agent reads the snip and turns the wall, and the person's Cmd+Z takes the turn back](docs/media/hero.gif)

The GIF above runs once through: a snip, an agent's turn, and an undo. The full recording is [`docs/media/demo.mp4`](docs/media/demo.mp4), 64 seconds long.

## Why

- **Layouts come from maps a person built.** The grammar is learned from the example, so the output keeps its look and its rules.
- **People and agents share one undo history.** A turn an agent makes is undone with the same Cmd+Z a person uses.
- **Runs are deterministic.** Every crate draws from one seeded ChaCha8 stream (`bevy_emerge_rng`), so the same seed gives the same layout.
- **The core is engine-free.** `bevy_emerge_core` has no Bevy dependency, and `crates/bevy_emerge_core/tests/engine_free.rs` checks it.

## Crates

| Crate | What it is | Engine |
|---|---|---|
| `bevy_emerge_core` | The engine-free world-building core: constraint IR, solvers, WFC and the deterministic RNG. | none |
| `bevy_emerge_snip` | Cuts one rectangle out of a PNG of the editor, shared by the editor and the MCP server. | none |
| `bevy_emerge` | The runtime half: `spawn_descriptor` and a plugin that put a library and a map into a Bevy world. | Bevy 0.20 |
| `bevy_emerge_anim` | Clip-weight blending on a skinned rig's `AnimationPlayer`. | Bevy 0.20 |
| `bevy_emerge_mapper` | The editor. It opens a project directory and writes what a game loads. It is AI-written and not line-by-line audited; read its [README](crates/bevy_emerge_mapper/README.md) before you rely on it. | Bevy 0.20 |
| `bevy_emerge_mcp` | An MCP server that lets an agent drive a running editor through typed operations. | Bevy Remote Protocol client |
| `bevy_emerge_rng` | One deterministic ChaCha8 stream, shared by every crate. | none |
| `bevy_emerge_solver` | A deterministic SAT back-end with no clock, threads or entropy. | none |
| `bevy_devshot` | A frame on demand: touch a sentinel file and the next frame is written as a PNG. | Bevy 0.20 |

Most crates have their own `README.md` and a `CLAUDE.md` that lists their invariants.

## Quickstart

You need a Rust toolchain that builds edition 2024 and a machine that can open a Bevy window. Then:

```sh
git clone https://github.com/Ladvien/bevy_emerge
cd bevy_emerge
cargo run -p bevy_emerge_mapper -- <project-root> [map-name]
```

- `<project-root>` is a directory that holds `assets/emerge/`, the kit and maps the editor opens.
- `map-name` is the map to open. Save with Cmd+S; Cmd+O goes back to the menu.

To let an agent drive a running editor, start the MCP server against the same project. Both ends read `BEVY_BRP_PORT`, which defaults to 15702 on each side:

```sh
cargo run -p bevy_emerge_mcp -- --project <project-root>
```

An MCP client config for it looks like this:

```json
{
  "mcpServers": {
    "bevy_emerge": {
      "command": "cargo",
      "args": ["run", "-q", "-p", "bevy_emerge_mcp", "--", "--project", "/path/to/project"],
      "cwd": "/path/to/bevy_emerge"
    }
  }
}
```

Run the editor and the server against the same project directory.

## Common tasks

### Place a piece and undo it

Pick a piece from the palette, click the grid to put it down, and press Cmd+Z to take it back. The editor's status lines say what the next press will do. The [placing and undo guide](docs/guide/placing-and-undo.md) has the details.

### Snip a region for an agent

Before: the agent cannot see what you are looking at. After: it reads your snip and its note, makes one change, and you can undo that change.

![Snip, agent turn and undo](docs/media/hero.gif)

1. Press Cmd+\ to arm the snip, then drag a box over the region.
2. Type a note. The agent reads it with `emerge_list_snips`.
3. The agent changes the placement with an operation such as `placement_set_yaw`.
4. Press Cmd+Z to undo the agent's change.

### Move the camera

Before: the view is fixed on one part of the town. After: holding A and D moves it across the town.

![The camera pans left and right across the town](docs/media/move-the-camera.gif)

The [framing and camera guide](docs/guide/framing-and-camera.md) covers framing and the camera in full.

## Agents

The MCP server gives an agent the same typed operations a person uses. Its tools are `emerge_list_operations`, `emerge_preview_operation`, `emerge_apply_operation`, `emerge_undo_operation`, `emerge_read_oplog`, `emerge_state`, `emerge_frame_view`, `emerge_capture_still`, `emerge_capture_sequence`, `emerge_snip` and `emerge_list_snips`. The [tool reference](docs/mcp-tools.md) lists their parameters.

An agent's change lands in the same operation log and the same undo history as a person's change.

Known gaps:
- Box and stamp removal have no agent operation yet.
- `emerge_state` does not list each pin.
- Renaming a map does not rewrite other references to it by name.

## How it fits a game

Sanctuary, a game built on Bevy, depends on `bevy_emerge_core` by path. It lays out town blocks with wave function collapse, learned from an authored example block. The editor runs against Sanctuary as a project.

## Documentation

- [Editor guide](docs/guide/README.md): every page of the editor's guide, from getting started to keys.
- [MCP tool reference](docs/mcp-tools.md).
- [The iteration loop](docs/guide/loop.md): how the editor's changes are proposed, proved and recorded.
- [Full walkthrough](docs/media/demo.mp4): 64 seconds of the running editor.

## Contributing

Run `cargo test --workspace` before you push. CI runs the same command on Linux (see the badge at the top). Most crates have a `CLAUDE.md` that lists their invariants, and `AGENTS.md` holds the rules for agents working in this repo.

## Licence

MIT OR Apache-2.0, at your option. See `LICENSE-MIT` and `LICENSE-APACHE`.

The licence covers the code. The models under `assets/` are test fixtures copied from foundation_vs_slop (where they were already public). Third-party models (Kenney, Quaternius, SCP community models and others) keep their own licences, and a folder's `README.md` says where it came from when it has one. The FiraMono font in `crates/bevy_emerge_mapper/assets/fonts/` is under the SIL OFL 1.1 (`FiraMono-OFL.txt`).
