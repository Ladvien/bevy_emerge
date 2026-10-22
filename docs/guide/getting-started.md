# Getting started

## Build and launch

```sh
cargo test --workspace                       # the gate the loop runs
cargo run -p bevy_emerge_mapper -- <project-root> [map-name] [--door <door>] [--kit <kit>]
```

- `<project-root>` is the directory that holds the project's kit and maps.
- `map-name` opens a map directly. The positional arguments and flags are parsed by `crates/bevy_emerge_mapper/src/args.rs`.
- `--door kit`, `--door map` or `--door rigs` opens that door. `--kit <kit>` names the kit.

## The menu

The menu lists the kits and maps in the project. Choosing one opens it in the door that matches.

**Cmd+O** goes back to the menu from a map. If the map has unsaved edits, it saves them first and goes back in the same press (`save_and_leave` in `crates/bevy_emerge_mapper/src/editor.rs`). A refused save keeps you on the map and says why.

**Esc** is the key that still asks before it leaves unsaved work behind.

## The doors

The editor has three doors:

- **Kit** makes reusable content, in three panels: **MESHES**, then **TILES**, then **COMPOSE**. A mesh is a part, a tile is a sub-assembly built from meshes, and a composition is built from tiles.
- **Map** builds the level out of the kit. It has one panel, **MAP**.
- **Rigs** is the rig bench, **ANIM**, for character assets rather than level content.

**`=`** switches to the other door. Within a door, **1**, **2** and **3** choose the panel. On the Map door there is only one panel, so those keys do nothing there.

## Saving

**Cmd+S** writes the map. `Project::save` validates the map before it writes, and the write is atomic (`ron_surgery::save_atomic`), so a map that cannot be written is refused rather than half-written. Nothing is written to the map file until you save or leave with Cmd+O.

**Cmd+E** lists every error this session has produced.

## When something is refused

A refusal goes to the status as a problem. It shows as a card in the viewport for a few seconds and stays in the journal, which **Cmd+E** opens. A click that does nothing says so, rather than being silently ignored. Holding **K** shows the keys that apply to the tab you are on; see [Keys](keys.md).
