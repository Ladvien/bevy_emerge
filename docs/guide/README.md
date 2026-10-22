# bevy_emerge editor guide

The editor is how a level designer builds a map out of a kit: the meshes, the tiles cut from them, and the compositions built from tiles. Everything here is what the editor does today; where a page describes a step, the step was checked against `crates/bevy_emerge_mapper/src/keys.rs` or the running editor.

| Page | What it covers |
|---|---|
| [Getting started](getting-started.md) | Building, launching, the menu, the two doors, the panels, saving |
| [Placing and undo](placing-and-undo.md) | Placing, turning, lifting, removing, moving, cloning, running a line, pinning, undo and redo |
| [Fills and proposals](fills-and-proposals.md) | Flood fills, the learned, declared and composed generators, and accepting or throwing a proposal away |
| [Kit: meshes, tiles, compositions](kit-meshes-tiles-compositions.md) | Building the reusable pieces a map is made of |
| [Framing and the camera](framing-and-camera.md) | Where the camera starts, panning, turning and zooming, and framing from an agent |
| [Agents and the MCP server](agents-and-mcp.md) | Driving the editor from an agent, with the same undo history a person uses |
| [Keys](keys.md) | Every key, when it applies, and what it does |
| [Demo](../media/demo.mp4) | The full 64-second walkthrough. The README shows the short GIFs |
| [The iteration loop](loop.md) | How the editor's changes are proposed, proved and recorded |

## Watch it first

Three recordings of the running editor on the scratch town map:
- [Snip, agent turn and undo](../media/hero.gif), 14 seconds: the person snips a wall and writes a note, and an agent reads it and turns the wall. The recording ends before the undo.
- [Moving the camera](../media/move-the-camera.gif), 6 seconds: holding A and D pans across the town.

The [full walkthrough](../media/demo.mp4), 64 seconds, shows the rest of this guide's path: open, frame, place and undo; an agent framing a region; an agent turning and removing placements, each taken back with Cmd+Z; and a person's snip read back with `emerge_list_snips`.

## Where things live

- `crates/bevy_emerge_mapper/` is the editor. `src/keys.rs` owns every binding; `src/editor.rs` owns the map tab.
- `crates/bevy_emerge_core/` holds the data formats and the placement stack.
- `crates/bevy_emerge_mcp/` is the MCP server that lets an agent drive a running editor. Its tools are listed in [`docs/mcp-tools.md`](../mcp-tools.md).
