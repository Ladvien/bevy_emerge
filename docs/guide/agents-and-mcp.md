# Agents and the MCP server

The editor can be driven by an agent through the Model Context Protocol. The server is a separate binary, `bevy_emerge_mcp`. It speaks MCP over stdio and reaches the running editor over the Bevy Remote Protocol.

The complete tool reference, with parameters and annotations, is generated from the server's own declarations: [`docs/mcp-tools.md`](../mcp-tools.md). This page explains how the tools fit together.

## Start the server

Start the editor on a project first, then point the MCP client at the server:

```sh
bevy_emerge_mcp --project <dir>
```

- `--project <dir>` is the directory the editor is working on. The server never writes to it; the editor does. It defaults to the current directory.
- The editor's BRP endpoint is read from `BEVY_BRP_PORT`, default `15702`. Set it the same for the editor and the server.

The project is checked when a tool is called, not when the server starts, so the server can start before the editor does.

## How an edit travels

Every change an agent makes is an **operation**: a named, typed action the editor knows how to apply and how to undo. The same operations back the keys, so an agent's edit and a person's edit are the same kind of thing.

1. `emerge_list_operations` lists every operation with its parameter schema and how it is undone. Call it before applying anything.
2. `emerge_preview_operation` says what would change, and changes nothing.
3. `emerge_apply_operation` applies it. The operation is written to the operation log.
4. `emerge_undo_operation` takes it back.

A refusal says why. For example, `placement_set_pin` refuses a pin with no reason.

## Tools

| Tool | What it does |
|---|---|
| `emerge_list_operations` | Lists the operations, with schemas and undo behaviour. Read-only. |
| `emerge_preview_operation` | Shows what an operation would change, without changing it. Read-only. |
| `emerge_apply_operation` | Applies one operation (`operation`, `parameters`). |
| `emerge_undo_operation` | Undoes an operation, including one an agent applied. |
| `emerge_state` | Reads the editor's current state. |
| `emerge_capture_still` | Captures one frame of the editor. |
| `emerge_snip` | Cuts a rectangle of the editor's picture, in window pixels, and returns it as an image. |
| `emerge_capture_sequence` | Captures frames over time (`label`, `seconds`, `fps`). |
| `emerge_read_oplog` | Reads the operation log. |
| `emerge_frame_view` | Frames the Map camera on the map, the selection, a set of placements, or a rectangle. |

## Pinning, turning and removing from an agent

`placement_set_pin` pins or unpins a placement. Pinning requires a `because` reason, and unpinning refuses one. A pinned placement survives a fill generate. `placement_set_yaw` turns a placed piece to a facing in degrees (wrapped into one full turn), which is what **R** and **T** do to a placed piece under the cursor. `placement_remove` removes a placement by id, and whatever stands on it goes too; `emerge_undo_operation` puts the removed pieces back at the rows they held. See [Placing and undo](placing-and-undo.md).

## Known gaps

- **Cmd+Z and an agent's edits share one history.** Cmd+Z takes back the newest change still in effect, an agent's included. `emerge_undo_operation` takes back a logged change by its operation id, or puts it back, but only when no newer change to the same map is still in effect: otherwise it refuses and names the change to undo first. See [Placing and undo](placing-and-undo.md#undo-and-redo).
- **`emerge_state` does not list each placement's pin.** Use `emerge_read_oplog` to see what was pinned, until E-026 is fixed.

E-026 is open in `docs/loop/backlog.json`.
