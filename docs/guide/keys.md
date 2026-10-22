# Keys

Every key the editor answers to, written from the `BINDINGS` table in `crates/bevy_emerge_mapper/src/keys.rs` as it stood when this page was last updated. If a binding changes in the code, this page changes with it. Holding **K** in the editor shows the live list for the tab you are on, and that list wins if the two disagree.

`Cmd` is the platform command key (`⌘` on macOS). Where a row says **Shift:** or a chord has `Shift+`, the modifier changes what the key does; a `left`/`right`/`up`/`down` row is an arrow key.

## Every tab

| Key | When | Does |
|---|---|---|
| `1` | — | panel 1 / 2 / 3 of this door |
| `2` | — | panel 1 / 2 / 3 of this door |
| `3` | — | panel 1 / 2 / 3 of this door |
| `Cmd+Delete` | — | define this piece |
| `K` | — | hold: every key lands on what it does |
| `Cmd+S` | — | save |
| `Cmd+O` | — | back to the menu |
| `=` | — | the other door |
| `Cmd+E` | — | every error this session |
| `Cmd+C` | — | copy this tab's text |
| `Esc` | — | back out |
| `W` | — | pan |
| `A` | — | pan |
| `S` | — | pan |
| `D` | — | pan |
| `Q` | — | turn view |
| `E` | — | turn view |
| `Cmd+F` | — | filter the list |
| `Cmd+\` | — | snip: press to arm (press again to disarm), drag a rectangle over the editor, type a note. Enter saves a picture and its record; Esc cancels and writes nothing |

## MAP

| Key | When | Does |
|---|---|---|
| `Cmd+Z` | — | undo / redo |
| `Shift+Cmd+Z` | — | undo / redo |
| `H` | — | target the stack |
| `up` | while nothing in hand | walk the palette / Shift: x5 |
| `down` | while nothing in hand | walk the palette / Shift: x5 |
| `P` | while nothing in hand | take the arrows to the compositions |
| `up` | while picking a composition | walk the compositions / Shift: x5 |
| `down` | while picking a composition | walk the compositions / Shift: x5 |
| `R` | — | turn L / turn R / tip x / tip z / straight |
| `T` | — | turn L / turn R / tip x / tip z / straight |
| `Y` | — | turn L / turn R / tip x / tip z / straight |
| `U` | — | turn L / turn R / tip x / tip z / straight |
| `V` | — | turn L / turn R / tip x / tip z / straight |
| `J` | — | grid rung: tile / fine / finer |
| `[` | — | lift / lower this |
| `]` | — | lift / lower this |
| `X` | — | removal mode |
| `Home` | — | frame the view: the locked placement, or the whole map |
| `B` | — | move / Shift: clone / Cmd: run / M: keep as a composition |
| `Shift+B` | — | move / Shift: clone / Cmd: run / M: keep as a composition |
| `Cmd+B` | — | move / Shift: clone / Cmd: run / M: keep as a composition |
| `M` | — | move / Shift: clone / Cmd: run / M: keep as a composition |
| `N` | — | rename map: type the name, then Enter moves the file now / Esc cancels |
| `O` | — | pin / unpin |
| `F` | while nothing in hand | fill: brush / learned / declared / composed |
| `G` | while nothing in hand | fill: brush / learned / declared / composed |
| `Shift+G` | while nothing in hand | fill: brush / learned / declared / composed |
| `Cmd+G` | while nothing in hand | fill: brush / learned / declared / composed |
| `Enter` | while a proposal is waiting | keep this layout / Esc throws it away |
| `Enter` | while picking a composition | arm this composition for the map |

On a Mac laptop without a Home key, press `Fn+←` for `Home`.

## KIT · MESHES

| Key | When | Does |
|---|---|---|
| `up` | — | walk the lists / Shift: x5 |
| `down` | — | walk the lists / Shift: x5 |
| `left` | — | walk the lists / Shift: x5 |
| `right` | — | walk the lists / Shift: x5 |
| `I` | — | type an id |
| `M` | — | mount |
| `Enter` | while nothing in hand | add / update this tile |
| `Space` | except while holding a piece | open / close the pack |
| `Enter` | while a proposal is waiting | keep the derived edges / Esc throws them away |
| `R` | — | rescan / exclude |
| `Shift+R` | — | rescan / exclude |
| `Delete` | — | send back to NOT IMPORTED |
| `Cmd+Z` | — | undo / redo |
| `Shift+Cmd+Z` | — | undo / redo |
| `T` | — | cell forward |
| `F` | — | cell left |
| `G` | — | cell back |
| `H` | — | cell right |
| `Shift+T` | — | add a vocabulary token |
| `[` | — | previous layer |
| `]` | — | next layer |
| `Z` | — | solid this cell |
| `X` | — | edge this cell |
| `V` | — | clear this cell |
| `B` | — | rescan solid / turn x / y / z |
| `N` | — | rescan solid / turn x / y / z |
| `O` | — | rescan solid / turn x / y / z |
| `P` | — | rescan solid / turn x / y / z |
| `L` | — | suggest / all / abandon |
| `Shift+L` | — | suggest / all / abandon |
| `Shift+Y` | — | suggest / all / abandon |
| `/` | — | filter the tags |

## KIT · TILES

| Key | When | Does |
|---|---|---|
| `up` | while browsing the tiles | walk the tiles / Shift: x5 |
| `down` | while browsing the tiles | walk the tiles / Shift: x5 |
| `right` | while browsing the tiles | to the meshes / left goes back |
| `left` | while nothing in hand | back to the tiles |
| `Enter` | while browsing the tiles | open this tile |
| `up` | while nothing in hand | walk the library / Shift: x5 |
| `down` | while nothing in hand | walk the library / Shift: x5 |
| `up` | while holding a piece | move the piece |
| `down` | while holding a piece | move the piece |
| `left` | while holding a piece | move the piece |
| `right` | while holding a piece | move the piece |
| `,` | while holding a piece | step to the previous / next member |
| `.` | while holding a piece | step to the previous / next member |
| `Shift+up` | while holding a piece | flush it to that side |
| `Shift+down` | while holding a piece | flush it to that side |
| `Shift+left` | while holding a piece | flush it to that side |
| `Shift+right` | while holding a piece | flush it to that side |
| `Space` | except while browsing the tiles | take the piece / Esc puts it back |
| `[` | except while browsing the tiles | layer |
| `]` | except while browsing the tiles | layer |
| `J` | except while browsing the tiles | grid deeper by thirds, wrapping |
| `Enter` | while nothing in hand | drop the piece / Shift: a slot |
| `Shift+Enter` | while nothing in hand | drop the piece / Shift: a slot |
| `R` | except while browsing the tiles | turn / remove this / Shift: empty the tile |
| `Delete` | except while browsing the tiles | turn / remove this / Shift: empty the tile |
| `Shift+Delete` | except while browsing the tiles | turn / remove this / Shift: empty the tile |
| `N` | — | name a new tile |
| `C` | — | close the tile |
| `Cmd+Z` | except while browsing the tiles | undo / redo |
| `Shift+Cmd+Z` | except while browsing the tiles | undo / redo |

## KIT · COMPOSE

| Key | When | Does |
|---|---|---|
| `up` | — | walk the focused list / Shift: x5 |
| `down` | — | walk the focused list / Shift: x5 |
| `Enter` | — | arm this composition for the map |
| `I` | — | rename this composition |
| `left` | — | which list the arrows walk |
| `right` | — | which list the arrows walk |
| `O` | — | previous / next composition |
| `P` | — | previous / next composition |

## RIGS · ANIM

| Key | When | Does |
|---|---|---|
| `up` | — | walk the rigs / Shift: x5 |
| `down` | — | walk the rigs / Shift: x5 |
| `Enter` | — | adopt measured values into rigs.ron |
| `Cmd+Z` | — | undo / redo the last write |
| `Shift+Cmd+Z` | — | undo / redo the last write |
| `left` | — | scrub phase (Shift: fine) |
| `right` | — | scrub phase (Shift: fine) |
| `Space` | — | play / scrub |
| `C` | — | check all rigs |
| `G` | — | ghost: measured over declared |
| `V` | — | view: figure / feet / side / ground |
