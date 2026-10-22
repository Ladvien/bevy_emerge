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
| [Demo](../../README.md#the-editor) | The recording in the README: an agent framing a region, then turning a placement |
| [The iteration loop](loop.md) | How the editor's changes are proposed, proved and recorded |

## Watch it first

The [README](../../README.md) embeds `docs/media/demo.mp4`, a 64-second recording of the running editor on the scratch town map. It shows the path this guide walks:
- open, frame, place and undo;
- an agent framing a region of the town;
- an agent turning a placement with `placement_set_yaw` and removing one with `placement_remove`, each taken back with Cmd+Z;
- the person snipping an area with Cmd+\ and a note, which an agent reads back with `emerge_list_snips`.

## Where things live

- `crates/bevy_emerge_mapper/` is the editor. `src/keys.rs` owns every binding; `src/editor.rs` owns the map tab.
- `crates/bevy_emerge_core/` holds the data formats and the placement stack.
- `crates/bevy_emerge_mcp/` is the MCP server that lets an agent drive a running editor. Its tools are listed in [`docs/mcp-tools.md`](../mcp-tools.md).
