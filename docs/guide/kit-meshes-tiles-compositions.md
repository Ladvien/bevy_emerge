# Kit: meshes, tiles, compositions

A kit is the set of reusable pieces a map is built from. The Kit door has three panels, and each builds on the one before it. The rig bench is a separate door, covered at the end.

Keys below are from the `Meshes`, `Tiles`, `Compose` and `Anim` contexts in `crates/bevy_emerge_mapper/src/keys.rs`. The full list is in [Keys](keys.md).

## MESHES (panel 1)

The Meshes panel turns imported `.glb` pieces into labelled parts. It has two lists, the **candidates** and the **library**.

- **Left** and **Right** choose which list the arrows walk (`FocusCandidates`, `FocusLibrary`). **Up** and **Down** walk it; **Shift** with either arrow moves five at a time.
- **/** filters the tags. **Cmd+F** filters the list.
- **Space** opens or closes a pack, a folder of meshes grouped together.
- **R** rescans a piece; **Shift+R** excludes a whole pack from the scan.
- **Delete** sends a piece back to NOT IMPORTED.
- **M** mounts a piece. **I** types an id for it.
- **Enter** adds the piece as a tile, or updates the tile it already is.

Labelling a piece is done cell by cell, on the grid the piece sits in:

- **T**, **F**, **G**, **H** set the cell forward, left, back and right.
- **[** and **]** move between layers.
- **Z** marks the cell solid, **X** marks it as an edge, **V** clears it.
- **Shift+T** adds a new token to the vocabulary the edges are described with.
- **B** rescans the solid cells, and **N**, **O**, **P** turn the mesh about its X, Y and Z axes.

Label suggestions are offered rather than applied:

- **L** suggests labels for this piece.
- **Shift+L** suggests for all of them.
- **Shift+Y** discards every suggestion.
- When a suggested set of edges is waiting, **Enter** keeps the derived edges and **Esc** throws them away.

**Cmd+Z** undoes a meshes edit, and **Shift+Cmd+Z** redoes it.

### The labels row

The INSPECTOR's labels row says where a piece's labels came from and how sure that source was. For a model it reads `confidence high · <model file> · <date>`: the confidence (high, medium or low) first, then the model file's name without its folder or extension, then the date the labels were made. Labels a person set read `by hand · <date>`. A long row wraps inside the panel. The labels file keeps the model's full path; only the row shortens it.

Labels a model suggested show in the unchecked colour until a person confirms each axis. A confirmed axis is drawn like the rest of the inspector.

An agent finds the same pieces without reading each one. `emerge_list_pieces` filters the list with `unlabelled` (no labels yet), `by_model` (labelled by a model or an agent, and not yet confirmed by a person), `confidence` (high, medium or low), `with_mesh` and `id_contains`. The reply counts every match, and it lists fifty at a time.

The kit door opens with `=`, and it opens on this Meshes panel. An agent crosses to it with the `door_open` operation.

## TILES (panel 2)

A tile is a sub-assembly: meshes set in a fixed arrangement so they act as one piece on the map. The Tiles panel has two pages, the tile list and the meshes it is made of.

On the **tile list** (browsing):

- **Up** and **Down** walk the tiles you have built.
- **Enter** opens the highlighted tile for editing.
- **Right** goes to the meshes page, and **Left** goes back.

On a **tile being built** (holding a piece):

- **Space** takes the piece up, and **Esc** puts it back.
- **Up**, **Down**, **Left**, **Right** move the piece on its grid.
- **Shift** with an arrow flushes the piece to that side of the tile.
- **,** and **.** step to the previous and next member of the tile.
- **[** and **]** change the layer, and **J** changes the grid, as on the Map door.
- **R** turns the piece. **Delete** removes this member; **Shift+Delete** empties the tile.
- **Enter** drops the piece where it is. **Shift+Enter** drops it into a slot.
- **Cmd+Z** and **Shift+Cmd+Z** undo and redo tile edits. This history is separate from the map's.

## COMPOSE (panel 3)

A composition is a set of tiles arranged as one unit on the map: a gatehouse, a row of houses. The Compose panel lists them.

- **Up** and **Down** walk the list. **Shift** with either arrow moves five at a time.
- **Left** and **Right** choose which list the arrows walk.
- **O** and **P** step to the previous and next composition.
- **I** renames the composition.
- **Enter** arms the composition for the map. The Map door then places it with a click; see [Placing and undo](placing-and-undo.md).

On the Map door, **P** takes the arrows to the compositions as well, and **Enter** arms the one highlighted.

## RIGS (ANIM)

The Rigs door checks character rigs. It is a separate door, opened with `=` or `--door rigs`.

- **Up** and **Down** walk the rigs.
- **Left** and **Right** scrub the phase of the animation; **Shift** scrubs finely.
- **Space** plays and pauses.
- **Enter** adopts the measured values into `rigs.ron`.
- **C** checks all rigs. **G** shows the ghost, measured over the declared pose. **V** cycles the camera preset: figure, feet, side, ground.
- **Cmd+Z** and **Shift+Cmd+Z** undo and redo the last write to the rig file.

## Where the data is written

Each write to the kit goes through the same atomic write as a map, so a failed write leaves the file as it was. A kit's compositions are saved to `compositions.ron`.
