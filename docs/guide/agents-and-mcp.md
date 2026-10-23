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

A refusal says why. For example, `placement_set_pin` refuses a pin with no reason. The person sees the refusal in the toast and the journal (Cmd+E). A change the agent applies replaces the person's receipt, the UNDER line in the inspector, so the person sees what changed.

## Tools

| Tool | What it does |
|---|---|
| `emerge_list_operations` | Lists the operations, with schemas and undo behaviour. Read-only. |
| `emerge_preview_operation` | Shows what an operation would change, without changing it. Read-only. |
| `emerge_apply_operation` | Applies one operation (`operation`, `parameters`). |
| `emerge_undo_operation` | Undoes an operation, including one an agent applied. |
| `emerge_state` | Reads the editor's current state. |
| `emerge_list_pieces` | Reads the library's pieces a page at a time: each id, its mesh, its kind, effects and looks, and who labelled each axis (`pieces_from`, `with_mesh`, `unlabelled`, `by_model`, `confidence`, `id_contains`). Read-only. |
| `emerge_capture_still` | Captures one frame of the editor. |
| `emerge_snip` | Cuts a rectangle of the editor's picture, in window pixels, and returns it as an image. |
| `emerge_capture_sequence` | Captures frames over time (`label`, `seconds`, `fps`). |
| `emerge_read_oplog` | Reads the operation log. |
| `emerge_frame_view` | Frames the Map camera on the map, the selection, a set of placements, or a rectangle. |
| `emerge_list_snips` | Lists the snips the person took, newest first, with the newest picture as an image. Reads the snips folder, so it works without an editor running. Read-only. |

## Pinning, turning and removing from an agent

`placement_set_pin` pins or unpins a placement. Pinning requires a `because` reason, and unpinning refuses one. A pinned placement survives a fill generate. `placement_set_yaw` turns a placed piece to a facing in degrees (wrapped into one full turn), which is what **R** and **T** do to a placed piece under the cursor. `placement_remove` removes a placement by id, and whatever stands on it goes too; `emerge_undo_operation` puts the removed pieces back at the rows they held. See [Placing and undo](placing-and-undo.md).

`emerge_state` names pinned placements by id and reason in `pinned`, up to 50 at a time: `pinned_total` is the full count, and past 50 `pinned_next` gives the `pinned_from` for the next page.

`stamp_remove {id}` removes a stamp, and `emerge_undo_operation` puts it back at its place. An agent's undo of a stamp removal waits while a person's stamp change by place sits above it in the history. The person takes that change back with Cmd+Z first, and the refusal says so.

`map_rename` renames the open map. The file moves with the name at once, and a name another map in the project already has is refused before anything is written. If the old file cannot be removed, the rename is refused and nothing changes: the new file is taken back and the map keeps its old name. If that take-back also fails, the map follows the new file and the refusal says so. **N** on the Map door dispatches the same operation, so a rename by an agent and a rename by a person are one kind of change, and Cmd+Z takes either back. An agent undoes a rename with `emerge_undo_operation`, which puts the old name back.

`brush_arm {descriptor}` arms a piece as the brush, the same as clicking its row in the palette or walking the palette with the arrow keys, which dispatch the same operation. It changes no map data, so it is logged but not undoable: `emerge_undo_operation` with its id says there is nothing to take back. `descriptor: null` puts the brush down, as Esc does. A piece the open map's bash hides is refused, naming the bash. An unknown id is refused with the closest library ids. `emerge_state` lists the library's ids in `descriptor_ids`, 50 at a time; `descriptor_ids_next` gives the `descriptors_from` for the next page.

`emerge_list_pieces` lists the pieces themselves, 50 at a time, in library order: `total` is the full count after the filters, and past 50 `pieces_next` gives the `pieces_from` for the next page. Each row carries `labels`: the record's model, date and confidence, and the axes it labels with who wrote each one (`model` or `human`). `with_mesh: true` leaves out the flat footprint markers (a road, a field) that name no mesh. `unlabelled: true` keeps only the pieces no one has labelled, and `by_model: true` keeps the pieces with an axis a model wrote and no person has confirmed. `confidence` (`high`, `medium` or `low`) and `id_contains` narrow further, and every filter combines with the others. When a filtered page is still capped, `how_to_narrow` names the filters.

## Snips the person took

A person snips an area of the editor with `Cmd+\` and types a note (see [Keys](keys.md)). The editor writes two files for each snip into `target/loop/captures/snips/`: a PNG of the cut-out rectangle, and a JSON record that names it. The record holds the note, the map, the active tool, the rectangle in window and picture pixels, the camera's view, the cursor's world position, and the placements inside the rectangle. `emerge_list_snips` reads that folder, newest first, and returns the newest picture as an image. A file there that is not a snip record, or a record with no picture, is listed under `not_snips` with the reason, and does not stop the list. A snip changes no map data: it is not an operation, and it is not logged or undoable.

The picture lands a frame or more after Enter. Until it does, the snip is listed under `pending` as "being saved", with its `age_ms`, and not under `snips`. A pending snip older than 10 seconds is listed as abandoned: the editor quit or the capture was lost, so take it again. When the editor opens a project, it clears every pending record an earlier editor left.

## Opening a door

`door_open {door, tab?}` crosses to a door: `kit`, `map` or `rigs`, and optionally a panel that door shows (`tab`, by label, e.g. `meshes`). It is the request the 1, 2 and 3 keys make, and it follows the same rules.

The reply's `outcome` is one of three words:

- `applied`: the door is open.
- `waiting`: the editor asks a question before it crosses. `waiting_on` says what it waits for. Today that is the leaving question, which comes up when the map has unsaved changes. `waiting` does not mean the door is open. `emerge_state` shows the question: `leaving` is true while it is up, and `door_waiting` names the door the crossing would reach.
- `refused`: nothing was requested, and `refusal` says why. A door request is refused while another one waits, and while a confirmation question is open.

The question gate applies to `door_open` only. Other operations still run while a question is open.

`log_entry` is the operation-log line written for the request. It records the request as accepted, so its outcome reads `applied` even when the reply says `waiting`. The crossing itself is not logged.

A door request has no undo. If its log line cannot be written, the request is refused.

## Known gaps

- **Cmd+Z and an agent's edits share one history.** Cmd+Z takes back the newest change still in effect, an agent's included. `emerge_undo_operation` takes back a logged change by its operation id, or puts it back, but only when no newer change to the same map is still in effect: otherwise it refuses and names the change to undo first. See [Placing and undo](placing-and-undo.md#undo-and-redo).
- **A box removal is one `placement_remove` per piece.** A person's drag box removes each piece by its own registered operation, so an agent that wants to remove a set calls `placement_remove` once for each piece. Each one is its own entry in the log, and Cmd+Z takes a person's whole box back in one step.
- **Other references to a map by name are not rewritten.** `map_rename` moves the file and changes the name in the map, and nothing else. The game loads maps by name, so anything that names the old map (a game-side reference, a config entry) has to be changed by hand.
- **A press that began before `Cmd+\` was pressed can still press a button under the drag.** While a snip is armed, picking is off, so a press that starts after arming cannot press a button. A press already in progress when the snip was armed is not covered.
