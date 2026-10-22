# Fills and proposals

A fill covers an area of the map in one gesture. The Map door has four fill keys. Three of them put up a proposal rather than changing the map; a proposal stays until you accept it or throw it away.

## The four keys

| Key | Kind | What it does |
|---|---|---|
| **F** | Flood | Fills the empty cells outward from the cursor. It stops at anything already placed and at the map's edge. |
| **G** | Learned | Learns the arrangement from what is already on the map, then fills the free cells with more of it. It continues your own corner of the map. |
| **Shift+G** | Declared | Reads the kit's declared edge tokens. It works on an empty map. |
| **Cmd+G** | Composed | Builds the layout from the kit's compositions, whole tiles rather than single pieces. |

Each generator says so by name when it cannot answer, rather than quietly using another one. A learned fill and a declared fill are not interchangeable, and a generator never silently switches which one it uses.

## Propose, then decide

**G**, **Shift+G** and **Cmd+G** do not change the map when pressed. They put up a proposal. While one is waiting:

- **Enter** keeps the layout.
- **Esc** throws it away and leaves the map as it was.

No second generate starts while a proposal is waiting, so one answer cannot be stacked on another you have not looked at.

Pinned placements survive a generate: the generator replaces only the unpinned pieces. See [Placing and undo](placing-and-undo.md) for pinning.

## The flood cap

A flood stops at a cap, and the cap refuses rather than pretending to fill more. Flooding again from the same cell cannot pass it, because that cell already holds a piece. Flood from another empty cell instead.
