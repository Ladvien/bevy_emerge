# Placing and undo

This page covers the Map door. Keys are the ones in `crates/bevy_emerge_mapper/src/keys.rs`; the full list is in [Keys](keys.md).

## Choose a piece

The palette lists the pieces the kit provides. **Up** and **Down** walk it, and **Shift** with either arrow moves five at a time. The armed piece is the one a click puts down.

**P** takes the arrows to the compositions instead, so you can choose a whole composition for the map. **Enter** arms the highlighted composition. Compositions are covered in [Kit: meshes, tiles, compositions](kit-meshes-tiles-compositions.md).

## Place

- **Click** to put the armed piece down, on the grid.
- **Hold Alt** and click to put down one piece exactly where the cursor is, not snapped to the grid.
- **R** and **T** turn the piece left and right, in 15-degree steps. Each step is saved to the map file at once and logged as the operation `placement_set_yaw`. **Y** and **U** tip it along the two horizontal axes. **V** puts it back to the rotation and tip it was authored at.
- **[** and **]** lift and lower the piece.
- **J** moves the grid to the next rung: tile, fine, finer. The fine rungs are 0.333 m and 0.111 m on the default lattice; tile is 1.000 m.

## Remove

**X** enters removal mode. A click removes the piece under the cursor. A drag draws a box and removes what is inside it. A click on bare ground with removal armed does nothing and says so.

## Move

**B** picks up the piece under the cursor. Move the cursor and click again to put it down.

**Shift+B** clones: drag a box to copy what is inside it, or click to stamp one copy.

## Run a line

**Cmd+B** draws a line of the armed piece. Drag from one end to the other and the piece repeats along the line at its own pitch, which is the piece's spacing and not the grid's.

## Keep a group as a composition

**M** keeps the selected group as a composition, so it can be placed again as one piece.

## Pin

**O** pins the placement under the cursor, or unpins it. A pinned placement survives a fill generate: the generator replaces only the unpinned pieces. Pinning asks for a reason, and the reason is stored with the pin.

Agents pin through the same operation, `placement_set_pin`, and must give a reason too. Agents turn through `placement_set_yaw`. See [Agents and the MCP server](agents-and-mcp.md).

## Undo and redo

**Cmd+Z** takes back the last edit on the map. **Shift+Cmd+Z** puts it back.

An agent's edits are logged too, and `emerge_undo_operation` takes one back. Cmd+Z does not see an agent's edit yet: after an agent applies `placement_set_pin` or `placement_set_yaw`, Cmd+Z reports "nothing to undo". This is open as B-019 in `docs/loop/backlog.json`. Until it is fixed, take an agent's edit back with `emerge_undo_operation`. A turn you make on the Map door is on both histories: Cmd+Z reverts it on the editor's stack, and the operation log keeps its own record of it.

## Stack

**H** cycles which placement layer the next edit is aimed at (`CycleTarget`). Placements stack; this key chooses which stack the edit lands on.
