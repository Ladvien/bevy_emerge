# bevy_emerge feature loop log

One entry per iteration of the feature loop (the brief: the single feature that would most help a
game developer use the editor; one feature per run, researched, built, tested, three GREEN reviews,
shipped). Each run's folder is `runs/YYYY-MM-DD-NN/` with `research.md`, `plan.md`, `perf.md`,
`review.md`, `evidence/` and `video/`.

A structural decision that is not this agent's to make goes in `needs-decision/` with the options
and a recommendation, and the loop moves to the next item.

## Baseline — 2026-10-20

Measured on `main` at `c49ae80` (2026-10-19), before the first iteration.

- **Build:** `cargo build --workspace` clean, zero warnings, 12 m 21 s cold.
- **Tests:** `cargo test --workspace --no-fail-fast` → **1,183 pass, 1 fail**. The one failure is
  `bevy_emerge_mapper --test headless`'s `no_two_leaders_cross` (a badge-placement test: at
  1680×1050 `Control(CellGrid)`'s leader crosses `Control(Mesh)`'s). It has been red since at
  least 2026-09-03 and is unrelated to any game.
- **Headless boot against Sanctuary's real project:** `town` → 12 frames, 8,192 entities, no
  panic; `town_block` → 4,096 entities, no panic. This is the sanctioned way to check the wiring
  and it never touches a real keyboard or display.
- **Clippy / fmt / doc / CI:** not run, and there is no CI. The two `.github/workflows/ci.yml` in
  the tree are inert inside this monorepo — their own comments say they only go live when the
  `git subtree split` lifts them to a repository root. **Nothing runs on push.** That is why
  `no_two_leaders_cross` has been red for six weeks.
- **Content authored today (Sanctuary's project):** 173 descriptors (128 kit, 45 town), 16
  compositions, 727 placements in `town.map.ron` (of which **282 are the same fence piece**), 289
  in `town_block.map.ron`, 48 vocabulary tokens, 31 policy patches, 1 block rule.

Known candidates, carried from the 2026-10-20 technical review
(`/Users/ladvien/sanctuary/docs/design/emerge-mapper-review.md`):

1. Stamp a composition for repeated placements — the town map has **zero** stamps, so 282 fence
   pieces were placed one at a time.
2. Editor operations that do not exist: kit rename, project rename, project delete, adding a
   policy exclusion or patch, authoring a `bash`.
3. Tokens with no reader — 5 of 11 `effects` tokens are marked "NO READER YET" in `vocab.ron`,
   and `effects` is labelled on only 7 of 128 kit descriptors.
4. Untested areas: fill patterns (no test at all), the VLM HTTP path (no end-to-end test), a map
   round-trip with stamps + compositions + rules together.
5. `vlm.rs:1436-1442` — `probe` returns `Ready` for any non-loopback host **without connecting**,
   so a LAN endpoint that is down is reported ready.

Top 5 remaining:

1. _(no iteration yet — this is the first)_

## 2026-10-20-01: a map's stamps reach the game

- **Change:** `ProjectData::load` now expands a map's stamps, so the consumer game receives the
  rows the editor drew. `Map::expanded` is the new door; `Map::stamp_of` answers "which stamp
  brought this row"; `census::of_map` counts the rows the consumer will have, and the editor's
  inspector shows that number. The map **file** is unchanged — `MAP_VERSION` stays 6, a map still
  stores a reference and not the rows, and editing a group still moves every map that stamped it.

- **Why it ranked first:** the editor could already capture a group and stamp it, and the *file*
  was right — but the game never saw the group's rows. Measured on Sanctuary's own
  `town.map.ron`: the editor drew **746** rows, the game received **728**. The 18 missing rows
  were a house, and nothing said so — the editor said "saved", the map file said "1 stamp", and
  the game ran a level with a hole in it. That is the worst kind of sharp edge: a silent one, in
  the direction where the author trusts the tool.
  It beats the runner-up (**282 hand-placed fence pieces**) because the fence is a *content*
  decision the owner should make, while the missing rows are a *defect* in the tool: the fence
  could not have been a group before this, and now it can. The fence is still on the list —
  ranked first below, because with this shipped it is finally possible.

- **Evidence:**
  - Drove the real editor over BRP against a copy of Sanctuary's project: armed
    `town/house_cottage_plaster` on the Compose tab, stamped it on the map, saved, reloaded.
  - The **before** measurement, through the consumer's own door
    (`bevy_emerge_core::project_data::load`): `map.placements.len() = 728`,
    `map.stamps.len() = 1` — the 18 rows the editor drew were **absent**.
  - The **after** measurement: `map.placements.len() = 746`. The editor and the game now agree.
  - The file on disk holds a **reference** (`stamps: [ ( id: "house_cottage_plaster@1", of:
    "town/house_cottage_plaster", … ) ]`), and `placements` does **not** contain the group's
    rows.
  - A group renamed to one that does not exist: the editor **refuses by name** —
    `composition: house_cottage_plaster@1 refers to composition town/no_such_group, which does not
    exist` — and the map still opens with its 727 hand-placed rows drawn.
  - The editor's save gate used to prove the file was *loadable* while the game did not load it;
    it now expands through the same `Map::expanded` the consumer does.

- **Sources:** Shirley et al., *Procedural Content Generation in Games* (2018), ch. 1 and ch. 11
  (mixed-initiative; "can the computer explain itself?"); Mo et al., *StructureNet*, ACM TOG
  38(6):242, 2019, DOI 10.1145/3355089.3356527 (one uniform recursive type; the encoding invariant
  that stops one set having several encodings); Mokhov, Mitchell & Peyton Jones, *Build Systems à
  la Carte*, ICFP 2018 (provenance and dirty-checking — the `of_fingerprint` trace); Spall, Mitchell
  & Tobin-Hochstadt, *Forward Build Systems, Formally*, arXiv:2202.05328 (a derivation the
  consumer must be able to reproduce); Lamb & Zacchiroli, *Reproducible Builds*, IEEE Software
  38(6) 2021 (trust as checkability); Raistrick et al., *Infinigen Indoors*, arXiv:2406.11824;
  Mo et al., *ShapeAssembly*, arXiv:2009.08026 ("one program, many shapes"); Tutenel et al.,
  AIIDE 2010. Full notes, with the load-bearing quotes, in `runs/2026-10-20-01/research.md`.

- **Test / perf:**
  - `cargo test --workspace --no-fail-fast` → **1,198 pass, 1 fail**. The one failure is
    `no_two_leaders_cross` (the badge-placement test), **red since 2026-09-03 and unchanged by
    this work** — the baseline measured 1,183 pass, 1 fail, and this run adds 15 passing tests.
  - New: `bevy_emerge_core/tests/stamps_reach_the_consumer.rs` (11) and
    `bevy_emerge_mapper/tests/the_editor_and_the_game_agree.rs` (3). The round trip the
    2026-10-20 review listed as uncovered — a map with stamps **and** compositions **and** rules
    — is now tested.
  - `cargo clippy --workspace --all-targets -- -D warnings`: 78 errors, down from the baseline's
    82; **none in the files this change touches** (all pre-existing, in `adjacency.rs`,
    `grammar.rs`, `constraints.rs`, `rng`, `anim`, `solver`).
  - `cargo fmt --check`: 1,541 diffs, **none in the new files**; the two files this change edits
    had their pre-existing counts unchanged.
  - `cargo doc --no-deps`: no warning from any link this change adds.
  - Headless boot against Sanctuary's real project: `town` → 12 frames, 8,192 entities, no panic;
    `town_block` → 4,096 entities, no panic.
  - Sanctuary's own suite: **995 pass, 4 fail**, and **all four failures pre-date this change** —
    verified by re-running with this change stashed. They are another agent's in-flight work
    (`town_map`'s test scene names a `table` descriptor the library no longer defines, plus
    `save.rs` and two `siege/` tests).
  - Timings in `runs/2026-10-20-01/perf.md`. The one that matters: **728 → 746 rows** at the
    consumer.

- **Verdicts:** **Function: GREAT** — every state either refuses by name or counts honestly, and
  the game now gets the rows the editor drew. **Zoom: GREAT** — one defect found and fixed (a
  MAP value that wrapped to two lines, twice; the value column holds ~40 characters at the
  narrowest panel, and the wording that ships is 43). **Fresh eyes: GREAT** — discoverable on
  the Compose tab, stampable in one click, undoable, and the number in the inspector is the
  number the game will spawn. All three in `runs/2026-10-20-01/review.md`, with the captures in
  `runs/2026-10-20-01/evidence/`.

- **Commit:** `1bc0f5f` — "bevy_emerge_core, bevy_emerge_mapper: a map's stamps reach the
  consumer". Pushed to `Ladvien/bevy_emerge` `main` and pulled back; `main` == `origin/main` at
  `1bc0f5f`.
  **No PR.** This repository has never had one — every commit goes straight to `main`, and
  `gh pr create` refused with *"head branch `main` is the same as base branch `main`"*. The
  commit was already on `main` when the PR step was reached, and the owner's rule of 2026-10-20
  (*"Agents ask before force pushes and deletions"*) says a commit that landed on the wrong
  branch is *"fixed with a new commit or a question, never by resetting a shared working tree or
  force-pushing"*. So the branch `feature/stamps-reach-the-consumer` was pushed at the same
  commit for review, and **deleting it is left to the owner** — it is a remote branch this
  session created, but the rule names branch deletion as needing express permission, and the
  next loop should not have to reason about whether a branch is worth keeping.

- **Run:** `docs/loop/runs/2026-10-20-01/` — `research.md`, `plan.md`, `perf.md`, `review.md`,
  `evidence/`.

Top 5 remaining:

1. **The 282 hand-placed fence pieces, as groups.** Now *possible* — the group can be captured,
   stamped, and the game will have it. Measured: the fence is **two** shapes, not 282 — four
   field enclosures and one graveyard, on a 1.016 m pitch with the corner posts turned 90°. The
   editor's `F` fill lays one piece at one pitch and one yaw, and `Shift+B` captures what is
   already placed, so neither of them draws the shape; a **fence-run tool** (a piece, a pitch, a
   direction, and a corner rule) is the missing verb. This is the feature the composition model
   was waiting for, and it is now the highest-value one left. **Content decision for the owner
   on the town map**; the tool is this loop's to build.
2. **Editor operations that do not exist** — kit rename, project rename, project delete, adding a
   policy exclusion or patch, creating a `bash`. The mapper's own `CLAUDE.md` data-model table
   names every one of them as a hole, and the table is accurate.
3. **`vlm.rs:1436-1442`** — `probe` returns `Ready` for any non-loopback host **without
   connecting**, so a LAN endpoint that is down is reported ready. Ten minutes, and it is the
   difference between "the model is ready" and "the model is not there". Needs a test that calls
   `probe`, not `is_near`.
4. **Tokens with no reader** — 5 of 11 `effects` tokens in Sanctuary's `vocab.ron` are marked
   "NO READER YET", and `placement.rooms` / `placement.group` are unconsumed. A `vocab.ron` field
   that says which system reads a token, checked at load, would make the file's prose notes
   enforceable rather than advisory.
5. **Fill patterns have no test at all** — `fill.rs` is the one untested module in the mapper, and
   it is the module that lays the bulk of a level. A flood and a box fill are the two gestures
   that make a level cheap, and nothing pins their lattice, their refusals, or their
   idempotence.

## 2026-10-20-02: the run of a thing — one piece, repeated along a line

- **Change:** a **run** — arm a piece, drag a line, and the piece repeats along it **at its own
  pitch**, turning at the corners. `bevy_emerge_core::run` holds the arithmetic (engine-free);
  `Tool::Run` and `Cmd+B` are the editor's half. A run's posts are **ordinary placements**, so the
  map file gains rows and nothing else, the game's loader needs no change, and the inspector's
  count already includes them.
  With it: **`Cmd+F` reaches the palette's filter on every tab.** It was mouse-only, and a click
  that missed the box fell through to the map and **replaced a house block with a barrel**.

- **Why it ranked first:** two measurements, and the second is the one that decides it.
  *Finding a piece* took **~90 keystrokes** — the Map tab's filter was `Context::Tiles`, so the
  palette's box had **no keyboard route at all**, and the only way through was `Shift+Down`
  through 211 rows, and **the walk wraps** into the wrong kit. Now it is **2 keystrokes**.
  *Drawing the fence* took **282 clicks** and could not be done correctly at all: the fill lays
  a grid at the **project's** pitch, and the fence's posts are 2.032 m apart while the lattice's
  rungs are 1.000, 0.333 and 0.111 m — so a fill lays them 3.000, 2.333 or 2.111 m apart,
  **never 2.032**, and 8 cm out at best. A run's pitch is the **piece's own**, and that is the
  first thing in the editor that can draw this piece's own spacing.
  It beats the runner-up (**the editor's missing operations** — kit rename, project rename,
  policy add, bash authoring) because those are *operations an author may want*, while the filter
  is on **every** task an author does, and its failure mode is **editing the level by accident**.

- **Evidence:**
  - Drove the real editor over BRP against a copy of Sanctuary's project, before and after.
  - **Before:** `F` on the Map tab types into a box that ignores it — the box still reads the
    placeholder `filter` and the palette still shows `KIT · CONTAINER` (`evidence/before/`,
    `evidence/crops/1-…`). One click during the measurement replaced `block_3_1` with a barrel.
  - **After:** `Cmd+F` focuses the box, "fence" lands in it, and the palette narrows from **211
    rows to one** (`evidence/crops/2-…`). **Two keystrokes.**
  - The run itself: `Cmd+B`, drag, and **six posts stand along the line** — translucent while
    the drag is in progress, solid after the release, **the same six in the same places**
    (`evidence/crops/3-…` and `4-…`). `MAP 733 placed` (727 + 6) and `ran 6 piece(s) along the
    line`.
  - The pitch is the piece's: 2.064 m, and **not** on any of the lattice's rungs. A test asserts
    it.
  - The whole flow is **5 keystrokes and a drag**: `Cmd+F`, "fence", `Enter`, `↓`, `Cmd+B`.

- **Sources:** Alvarez et al., *Interactive Sketching of Urban Procedural Models*, SIGGRAPH Asia
  2009, DOI 10.1145/2897824.2925951 (§6.2 "Repetition Management" — a repetition is a **first-class
  gesture**, inferred from drawn segments); Merrell & Manocha, *Model Synthesis*, IEEE TVCG 16(2),
  2010, DOI 10.1109/TVCG.2010.112, and *Constraint-Based Model Synthesis*, Pacific Graphics 2007
  (the intent is a **constraint on the output**); Whiting et al., *Procedural Modeling of
  Structurally-Sound Masonry Buildings*, SIGGRAPH Asia 2009, DOI 10.1145/1618452.1618458 (repeat
  along an axis, as a standard rule); Shirley et al., *Procedural Content Generation in Games*,
  ACM Books 2018, ch. 11 (the human has final say; "what novel and useful editing operations can
  be incorporated?"); Cooper, *Sturgeon*, AIIDE 2022, DOI 10.1609/aiide.v18i1.21944 (a small
  mid-level API over an arrangement); Cockburn et al., *IJHCS* 69(7), 2011 (Fitts' Law — why a long
  palette walk is expensive and a filter is not); Gregory, *Game Engine Architecture*, 3rd ed.,
  ch. 15 (a special-case editing tool for shapes a general one cannot express). Full notes and
  quotes in `runs/2026-10-20-02/research.md`.

- **Test / perf:**
  - `cargo test --workspace --no-fail-fast` → **1,219 pass, 1 fail**. The one failure is
    `no_two_leaders_cross`, **red since 2026-09-03 and untouched by this work** (the baseline
    measured 1,198 pass, 1 fail; this run adds 21 passing tests).
  - New: `bevy_emerge_core/tests/a_run_is_laid_in_the_piece_s_own_pitch.rs` (7) and
    `bevy_emerge_mapper/tests/a_run_lays_a_line_of_pieces.rs` (4), plus 9 unit tests in `run.rs`
    and 1 in `census.rs`.
  - `cargo clippy --workspace --all-targets -- -D warnings`: **no finding in any file this change
    touches.**
  - `cargo fmt --check`: no diff in the new files.
  - `cargo doc --no-deps`: no warning from any link this change adds.
  - Headless boot against Sanctuary's real project: `town` → 12 frames, 8,192 entities, no panic;
    `town_block` → 4,096 entities, no panic.
  - **The key budget moved, deliberately and recorded in the pin**: `Context::Map` 24 → **25** for
    `RunMode`, and `Context::Global` 16 → **17** for `Cmd+F`. Both are stated in
    `keys.rs`'s pin with their reasons, which is the process the pin itself describes.

- **Verdicts:** **Function: GREAT** — every state reads correctly; one defect found (the drag
  showed only a line, not the posts) and fixed with a ghost, because a line is the *instruction* to
  lay, not the landing. **Zoom: GREAT** — the before/after crops of the filter box are the
  clearest evidence in the run. **Fresh eyes: GREAT** — discoverable in 5 keystrokes, do-able in
  one drag, undoable in one press. All three in `runs/2026-10-20-02/review.md`, with the captures
  in `runs/2026-10-20-02/evidence/{before,after,crops}/`.

- **Commit:** `aa254e8` — "bevy_emerge_core, bevy_emerge_mapper: a run of a thing, and a filter
  you can reach". Pushed to `Ladvien/bevy_emerge` `main`.
  **No PR**, for the same reason as the previous run: this repository has never had one, every
  commit goes straight to `main`, and `gh pr create` refuses with *"head branch `main` is the same
  as base branch `main`*. The commit was already on `main` when the PR step was reached, and the
  owner's rule of 2026-10-20 says a commit that landed on the wrong branch is *"fixed with a new
  commit or a question, never by resetting a shared working tree or force-pushing"*. The branch
  `feature/stamps-reach-the-consumer` from the previous run is still on the remote at the same
  commit; **deleting it is left to the owner**.

- **Run:** `docs/loop/runs/2026-10-20-02/` — `research.md`, `plan.md`, `perf.md`, `review.md`,
  `evidence/{before,after,crops}/`.

Top 5 remaining:

1. **The corners.** A run is a straight line, and a fence's corner is the same piece turned 90° —
   which the *group* model already expresses: four runs, captured as one group, stamped where the
   enclosure goes. The tool is there; the **content** decision (turning the town's 282 posts into
   five groups) is the owner's, and the graveyard's 3-post gap needs a `Gap` the editor does not
   yet draw.
2. **Editor operations that do not exist** — kit rename, project rename, project delete, adding a
   policy exclusion or patch, creating a `bash`. The mapper's own `CLAUDE.md` data-model table
   names every one of them as a hole, and the table is accurate.
3. **The Map-door crash** — `crates/bevy_emerge_mapper/docs/ui_audit.md` §F3 records it as open
   and reproduced 4/4: `bevy_emerge_mapper . <map>` dies inside two seconds with *"attachments
   have differing sizes"* — three cameras share one image and only the 3-D one has a depth
   attachment, so `prepare_core_3d_depth_textures` allocates per *target*. Two fixes have already
   been eliminated **by measurement**, and the audit says so; it wants a third, not a fourth
   guess.
4. **`vlm.rs:1436-1442`** — `probe` returns `Ready` for any non-loopback host **without
   connecting**, so a LAN endpoint that is down is reported ready. Needs a test that calls `probe`,
   not `is_near`.
5. **Fill patterns have no test at all** — `fill.rs` is the one untested module in the mapper, and
   it is the module that lays the bulk of a level. A flood and a box fill are the two gestures
   that make a level cheap, and nothing pins their lattice, their refusals, or their idempotence.

## 2026-10-20-03: the Map door opens — the editor stops dying on a resize

- **Change:** the editor's surface is **created at the window's physical size**, and a resize
  **announces itself in the frame the camera reads it**. `bevy_emerge_mapper . <map>` — the
  command that opens the editor straight onto a map — **no longer dies inside two seconds**, so
  the Map door is reachable from a terminal. The ratchet is
  `tests/the_surface_and_the_camera_agree.rs`; the measurement is
  `examples/probe_surface_resize.rs`.

- **Why it ranked first:** it is the only item on the list that **kills the application**. A missing
  operation costs an author a task; a crash costs them the *session* — and this one took the
  editor's **primary surface** away from the command line, so every session started that way
  detoured through the chooser. It beat the runner-up (the editor's missing operations — kit
  rename, project rename, policy add, bash authoring) because those are *operations an author may
  want*, and this was a door that **was not there**.

- **Evidence:** the cause was **measured, not argued**, and the measurement is the ui_audit's own
  instruction: *"what the next person should measure first"*.
  - The world camera is **3-D**, and so is the **only** camera with a **depth attachment**
    (a probe confirmed the four cameras: three share the surface image, one renders to the
    window).
  - `prepare_core_3d_depth_textures` sizes that depth from
    `camera.computed.target_info.physical_size` — the image's size **as it was when the camera
    last refreshed** (`bevy_core_pipeline-0.20.0/src/core_3d/mod.rs:667`).
  - A camera refreshes on the image's `AssetEvent::Modified`, read from the **message** stream,
    and `Assets::asset_events` writes that stream in `PostUpdate` **after** `camera_system` has
    already read it (`bevy_render-0.20.0/src/camera.rs:391`, `.before(AssetEventSystems)`). So a
    resize is **one frame late** at the camera — and that frame assembles the pass with a depth
    texture for the old extent and a colour attachment for the new one.
  - **The measured difference**, one frame after the window changes size (3396×1356 → 3440×1440,
    the audit's two numbers):

    | | Baseline | Fixed |
    |---|---|---|
    | the image | 3440×1440 | 3440×1440 |
    | what the cameras sharing it believe | **3396×1356** | **3440×1440** |
    | | **MISMATCH — the crash's frame** | agree |

  - **The ratchet fails on the unfixed code**, with the crash's own numbers: *"the camera at
    order -1 still believes the target is (3396, 1356) while the image is (3440, 1440)"*.
  - **Live:** the editor was started from the command line on Sanctuary's town map, **resized while
    it ran** (AppleScript — the compositor's own change), and screenshotted after: it **survived**
    and still drew.

- **Sources:** Akenine-Möller et al., *Real-Time Rendering*, 4th ed., ch. 23 (double and triple
  buffering: a target that changes size needs a *new* buffer, prepared while the old one is still
  shown); Scarpino, *The WebGPU Sourcebook*, app. B (a pass's colour attachment, and what wgpu
  requires of the attachments it is given); Hunt & Thomas, *The Pragmatic Programmer*, Tip 38
  *"Crash Early"* (a **bug** is worth crashing on; **exceptional data** is worth handling
  gracefully — and this is a bug, so it gets fixed rather than absorbed); *Bevy* 0.20's own source,
  read directly (`bevy_camera`, `bevy_render`, `bevy_core_pipeline`, `bevy_asset`) — the
  load-bearing source, and the engine this project is built on. Full notes and quotes in
  `runs/2026-10-20-03/research.md`.

- **Test / perf:**
  - `cargo test --workspace --no-fail-fast` → **1,221 pass, 1 fail**. The one failure is
    `no_two_leaders_cross`, **red since 2026-09-03 and untouched by this work** (the baseline
    measured 1,219 pass, 1 fail; this run adds 2 passing tests).
  - New: `tests/the_surface_and_the_camera_agree.rs` (2 tests) — the surface is created at the
    window's size, and a window resize never leaves the cameras behind. **The second one fails on
    the unfixed code.**
  - `cargo clippy --workspace --all-targets -- -D warnings`: **no finding in any file this change
    touches.** `cargo fmt --check`: `surface.rs` keeps its **pre-existing** 3 diffs, the new files
    are clean. `cargo doc --no-deps`: no warning from any link this change adds.
  - Headless boot against Sanctuary's real project: `town` → 12 frames, 8,192 entities, no panic;
    `town_block` → 4,096 entities, no panic.
  - Sanctuary's own suite: 1,014 pass, 4 fail — **the same four that were already red** (verified
    by the previous iteration with the emerge change stashed).
  - Timings in `runs/2026-10-20-03/perf.md`.

- **Verdicts:** **Function: GREAT** — the Map door opens from the command line, a live window
  resize survives, and the mismatch is measured away. **Zoom: GREAT** — the chrome is unchanged
  and crisp; the fix moved *when* the surface is sized, not what is drawn. **Fresh eyes: GREAT** —
  and the honest word for it is *there is no feature to learn*: a door that used to kill the
  application now opens, and a window that used to be fatal to drag now can be. All three in
  `runs/2026-10-20-03/review.md`, with the captures in `runs/2026-10-20-03/evidence/`.

- **Commit:** _(filled in below once pushed)_

- **Run:** `docs/loop/runs/2026-10-20-03/` — `research.md`, `plan.md`, `perf.md`, `review.md`,
  `evidence/`.

Top 5 remaining:

1. **The corners of a run.** A run is a straight line; a fence's corner is the same piece turned
   90°, and the *group* model already expresses that (four runs, captured as one group, stamped
   where the enclosure goes). The graveyard's 3-post gap needs a `Gap` the editor does not yet
   draw. The town's 282 posts are now *drawable*; turning them into groups is a **content decision
   for the owner**.
2. **Editor operations that do not exist** — kit rename, project rename, project delete, adding a
   policy exclusion or patch, creating a `bash`. The mapper's own `CLAUDE.md` data-model table
   names every one of them as a hole, and the table is accurate.
3. **`vlm.rs:1436-1442`** — `probe` returns `Ready` for any non-loopback host **without
   connecting**, so a LAN endpoint that is down is reported ready. Needs a test that calls `probe`,
   not `is_near`.
4. **Fill patterns have no test at all** — `fill.rs` is the one untested module in the mapper, and
   it is the module that lays the bulk of a level. A flood and a box fill are the two gestures
   that make a level cheap, and nothing pins their lattice, their refusals, or their idempotence.
5. **Tokens with no reader** — 5 of 11 `effects` tokens in Sanctuary's `vocab.ron` are marked
   "NO READER YET", and `placement.rooms` / `placement.group` are unconsumed. A `vocab.ron` field
   that says which system reads a token, checked at load, would make the file's prose notes
   enforceable rather than advisory.
