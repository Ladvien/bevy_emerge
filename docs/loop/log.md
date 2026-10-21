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

- **Commit:** `6938b97` - "bevy_emerge_mapper: the Map door opens, and a resize does not kill the
  editor". Pushed to `Ladvien/bevy_emerge` `main`.
  **No PR**, for the same reason as the previous two runs: this repository has never had one,
  every commit goes straight to `main`, and `gh pr create` refuses with *"head branch `main` is
  the same as base branch `main`"*. The commit was already on `main` when the PR step was
  reached, and the owner's rule of 2026-10-20 says a commit that landed on the wrong branch is
  *"fixed with a new commit or a question, never by resetting a shared working tree or
  force-pushing"*. The branch `feature/stamps-reach-the-consumer` from the run before is still
  on the remote at the same commit; **deleting it is left to the owner**.

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

## 2026-10-20-04: the arm crosses the door — the map can arm a composition

- **Change:** the arm — *which composition the next click stamps* — is **its own resource**,
  `compose::Armed`, classified `Ownership::Session` so it **survives a door change**. And the **Map
  door grew a COMPOSITIONS picker**: `P` takes the arrows, the arrows walk the project's
  compositions, and `Enter` arms one. **A house is on the map in four keystrokes and a click, on the
  door where stamping happens.**

- **Why it ranked first:** it was the only item on the list that was **structurally impossible**
  rather than merely missing. Every other candidate is a thing an author may want; this was a
  thing the editor could not do at all, and no content decision fixes it. Measured on 2026-10-21:
  the COMPOSE tab armed a house and said *"the map tab stamps it"* — **and there was no map tab in
  that process**, because the door is chosen at startup. The arm was a field of `ComposeState`,
  which `Ownership::Door` resets, so the only route to a stamp was to leave, come back through the
  menu, re-open the map, and **re-arm by hand**. It beat the runner-up (the 282 hand-placed
  fences) because those are a **content** problem the game's own `rules.ron` already answers at
  runtime, and the last iteration's log called it *"a content decision for the owner"*.

- **Evidence:**
  - **Sanctuary's own data**, parsed: `town.map.ron` holds **727 placements, 82 distinct
    descriptors, 282 of them `prop_wooden_fence_single`, and 0 stamps**. `compositions.ron`
    defines **16 house assemblies**, each with 18–38 members. **Not one was stamped on either
    map.**
  - **The cause, read from the source and measured.** `arm` is gated on `in_compose_mode`
    (`compose.rs`), `Mode::Compose` is in `Door::Kit.tabs()` and **only** there (`tiles.rs:113`),
    the door is a `Res<Door>` chosen at startup (`args.rs:104`), and `ComposeState` is
    `Ownership::Door` — so the arm is reset by the teardown (`screen.rs`).
  - **The fix:** the arm is its own resource, `Ownership::Session`; the Map door has a picker; the
    picker has a **keyboard** route, because `bevy_picking` reads the mouse from the `WindowEvent`
    stream (`bevy_picking-0.20.0/src/input.rs:153`) and BRP injects into
    `ButtonInput<MouseButton>` — **so no `Button` in this editor receives a click over BRP**, the
    palette's rows and the back button included. It was measured, not assumed, and the editor's
    own brief says the work is to be done *"by the keyboard, as key strokes are faster"*.
  - **The live result**, on Sanctuary's town map: `P` → the status reads *"the arrows walk the
    compositions — Enter arms one"*; `down` `down` → the cursor is on
    `town/house_cottage_wood`; `Enter` → *"`town/house_cottage_wood` armed — the next click stamps
    it"*; a click on the map → **`745 placed (727 hand, 18 group(s)), unsaved`** and *"stamped
    'town/house_cottage_wood' as house_cottage_wood@1"*. **The 18 group rows are that
    composition's 18 members.**
  - **The ratchet, on the broken code.** `tests/the_arm_survives_the_door_change.rs` is written so
    it **compiles on both** the baseline and the fixed code, and it **fails on the baseline**:
    *"the armed composition is a field of `ComposeState`, and `ComposeState` is `Ownership::Door` —
    so a door change resets it."*

- **Sources:** Shneiderman, *Direct manipulation for comprehensible, predictable and controllable
  user interfaces* (CHI 1997, `10.1145/238218.238281`) — direct manipulation depends on *"rapid
  incremental reversible operations whose effect on the object of interest is immediately
  visible"*, and an arm that is thrown away by a door change is none of the three. Reason,
  *Human Error: Models and Management* (1990) and *Experimental Slips and Human Error* (1992) — a
  slip is a description error, and the editor's own words create one: *"the map tab stamps it"*
  named a tab that did not exist in that process. Dwyer, Marriott & Wybrow, *Dunnart: A
  Constraint-Based Network Diagram Authoring Tool* (2009, `10.1007/978-3-642-00219-9_41`) — an
  instance must be creatable **from the thing it instances**, at the point of use. Sandhu, Chen &
  McCoy, *Enhancing wave function collapse with design-level constraints* (2019,
  `10.1145/3337722.3337752`) — the constraint is expressed in the author's terms, and Sanctuary's
  `rules.ron` already does that for the blocks. Full notes and quotes in
  `runs/2026-10-20-04/research.md`.

- **Test / perf:**
  - `cargo test --workspace --no-fail-fast` → **1,229 pass, 1 fail**. The one failure is
    `no_two_leaders_cross`, **red since 2026-09-03 and untouched by this work** (the baseline
    measured 1,221; this run adds 8).
  - New: `tests/the_arm_crosses_the_door.rs` (6 tests) and
    `tests/the_arm_survives_the_door_change.rs` (2 tests, the baseline proof).
  - `cargo test -p bevy_emerge_mapper --lib keys::` → 32 pass, **including
    `the_key_space_has_no_collisions`**: the four new census rows cost **no new key** except `P`.
  - `cargo clippy --workspace --all-targets -- -D warnings`: **no finding in any file this change
    touches.** `cargo fmt --check`: `compose.rs` and `keys.rs` keep their **pre-existing** diff
    counts (94 and 22); the new test files are clean. `cargo doc --no-deps`: **24 unresolved-link
    warnings, the same 24 as the baseline** — this change adds none.
  - Headless boot against Sanctuary's real project: `town` → 12 frames, 8,192 entities, no panic;
    `town_block` → 4,096 entities, no panic.
  - Sanctuary's own suite: 1,014 pass, 4 fail — **the same four that were already red**.
  - Timings in `runs/2026-10-20-04/perf.md`.

- **Verdicts:** **Function: GREAT** — the Map door grows a picker; `P`, the arrows and `Enter` arm a
  composition; the click stamps it, and the map reads `745 placed (727 hand, 18 group(s))`.
  **Zoom: GREAT** — the cursor and the arm are two marks in two inks, the type is crisp at 4×, and
  nothing else in the panel moved. **Fresh eyes: GREAT** — hold `K` and the keys say what they do;
  the arrows announce their new meaning the instant it changes; four keys and a click is the whole
  task. All three in `runs/2026-10-20-04/review.md`, with the captures in
  `runs/2026-10-20-04/evidence/`.

- **Commit:** `a09fcc2` - "bevy_emerge_mapper: the arm crosses the door, and the map can arm a
  composition". Pushed to `Ladvien/bevy_emerge` `main`.
  **No PR**, for the same reason as the previous three runs: this repository has never had one,
  every commit goes straight to `main`, and `gh pr create` refuses with *"head branch `main` is
  the same as base branch `main`"*. The commit was already on `main` when the PR step was
  reached, and the owner's rule of 2026-10-20 says a commit that landed on the wrong branch is
  *"fixed with a new commit or a question, never by resetting a shared working tree or
  force-pushing"*. The branch `feature/stamps-reach-the-consumer` from the run before is still
  on the remote at the same commit; **deleting it is left to the owner**.

- **Run:** `docs/loop/runs/2026-10-20-04/` — `research.md`, `plan.md`, `perf.md`, `review.md`,
  `evidence/`.

Top 5 remaining:

1. **A door switch.** The Kit and Map doors are **one application** — `screen.rs` says so, and the
   reason there is no Kit↔Map key has expired: it was declined because the alternative was a
   *relaunch*, and the relaunch is gone. The picker removes the *need* to cross, but an author who
   makes a house and then wants to see it on the map still leaves through the menu. The machinery
   exists (`args::open` is a pure function; `Opened::insert_into` puts the resources in); it wants a
   key, a refusal, and the unsaved-edit question.
2. **The 282 hand-placed fences.** Measured, and the largest single number in the map. But
   `rules.ron` already says the *game* lays the blocks out, and the fences are the *border* of a
   block — so this is content, and the last two iterations called it the owner's.
3. **Editor operations that do not exist** — kit rename, project rename, project delete, adding a
   policy exclusion or patch, creating a `bash`, composition delete/rename. A scout of
   `bevy_emerge_core` found **no function for any of the six** — not even one without a UI caller.
   Each needs a new on-disk writer with its own re-parse-before-save discipline, and the scout
   noted that a `Patch` has a multi-line `because` and a nested `Descriptor`, so `LineDoc::append` is
   not a direct fit.
4. **`fill.rs` has no test at all** — 1,058 lines, the one untested module in the mapper, and the
   module that lays the bulk of a level. A flood and a box fill are the two gestures that make a
   level cheap, and nothing pins their lattice, their refusals, or their idempotence.
   *(**Corrected 2026-10-20-07b: the module has 20 inline tests, and two of them bracket the
   defect exactly — one says a fill truncates and says so, the other says a fill starting on
   something is refused — and neither asks what the **next** fill does. The claim was stale.)*
5. **`vlm.rs:1436-1442`** — `probe` returns `Ready` for any non-loopback host **without
   connecting**, so a LAN endpoint that is down is reported ready. Needs a test that calls `probe`,
   not `is_near`.

## 2026-10-20-05: the door switch — the editor's two halves, and the key between them

- **Change:** **one key crosses between the Kit door and the Map door**, and the crossing **keeps
  the project**. The door strip names the other door beside the current one, and the key is `=`.
  **A house is on the map, and the house's own door is one key away — with no menu, no column
  walk, and no project reload.**

- **Why it ranked first:** it was the only item on the list that was **structurally impossible**
  rather than merely missing. Every other candidate is a thing an author may *want*; this was a
  thing the editor could not *do* — and no content decision fixes it. Measured on 2026-10-21: the
  round trip was **`Cmd+O`, `right`, `Enter`** each way — three keystrokes and a **full project
  reload** — to move between two halves of one project. It beat the runner-up (the six missing
  editor operations — kit rename, project rename, project delete, bash creation, policy patch add,
  composition rename/delete) because that is **six features**, each needing a new on-disk writer,
  while the door switch is **one** — and it is the one every one of the other six would be *done
  from*: an author renames a kit on the Kit door and then wants to see the map that uses it.

- **Evidence:**
  - **The round trip, measured**: `Cmd+O`, `right`, `Enter` — three keystrokes and a **full
    project reload** each way. The reload re-reads `kits.ron`, `vocab.ron` and `rules.ron`,
    re-validates **332 library descriptors** and **16 compositions**, and **re-measures every
    GLB's triangle count** by reading its JSON chunk. None of that changes when the author walks
    from one door to the other to look at the thing they just made.
  - **The `Ownership` table already said so**: `Project` is *"reloaded when the kit is not"*, and a
    same-kit crossing is not a kit change. **The crossing obeys the table.**
  - **The proof, over three crossings** (the editor's own log prints a line per project read):
    **project reads 2 before, 2 after**; map reads 2 before, 3 after — the map is re-read once,
    because one crossing went *to* the Map door, and **a map is a map**.
  - **The teardown is still a teardown**: every entity the old door made is gone, and every
    `Ownership::Door` resource is back at its opening value. `tests/the_door_switch_crosses.rs`
    says so — and **found its own bug**: the first version passed while the door's working state
    *survived*, because the headless harness never registered `ScreenPlugin` and so the teardown had
    never run. **The harness registers it now.**
  - **The live check**: three crossings driven over BRP on Sanctuary's town map — **zero panics**,
    and the project-read count did not move.

- **Sources:** Braem & Egner, *Getting a grip on cognitive flexibility* (2018,
  `10.1177/0963721418787475`) — a task set is *"a configuration of context-dependent production
  ('if, then') rules"*, and a door is one; the **switch cost** is the metric, and it is *"slower and
  less accurate performance on task switches than task repetitions"*; and the design instruction is
  that a cue should be *"a general readiness to switch between tasks rather than to switch to one
  particular alternative task"* — which is why the key says **"the other door"** and never names a
  direction. On Crump & Logan (2010), from the same paper: *"switch costs can be reduced for stimuli
  that are presented at a screen location associated with a higher likelihood of task switches"* —
  which is why the cue is a **place** (the door strip) and not a menu. Dourish & Bellotti, *Awareness
  and coordination in shared workspaces* (1992, `10.1145/143457.143468`) — awareness carried
  **passively by the workspace itself**, which is what the strip is. Ishii, *TeamWorkStation* (1990,
  `10.1145/99332.99337`) — the **cognitive seam** between two workspaces, and the shared surface
  that reduces it. Cockburn et al., *Supporting Novice to Expert Transitions in User Interfaces*
  (2014, `10.1145/2659796`) — **intermodal improvement**: a *mode* with a higher ceiling, which is
  why the crossing is a different action from the menu's and not a second way to do one thing. Full
  notes and quotes in `runs/2026-10-20-05/research.md`.

- **Test / perf:**
  - `cargo test --workspace --no-fail-fast` → **1,234 pass, 1 fail**. The one failure is
    `no_two_leaders_cross`, **red since 2026-09-03 and untouched by this work** (the baseline
    measured 1,229; this run adds 5). **The suite was run three times, with the same result each
    time** — no flaky test.
  - New: `tests/the_door_switch_crosses.rs` (5 tests).
  - `cargo test -p bevy_emerge_mapper --lib keys::` → 32 pass, **including
    `the_key_space_has_no_collisions`** — so the new row costs no key.
  - `cargo clippy --workspace --all-targets -- -D warnings`: **no finding in any file this change
    touches.** `cargo fmt --check`: every file this change touches keeps its **pre-existing** diff
    count, and `args.rs` drops from 1 to 0. `cargo doc --no-deps`: **25 unresolved-link warnings,
    the same 25 as the baseline** — this change adds none.
  - Headless boot against Sanctuary's real project: `town` → 12 frames, 8,192 entities, no panic;
    `town_block` → 4,096 entities, no panic.
  - Sanctuary's own suite: 1,014 pass, 4 fail — **the same four that were already red**.
  - Timings in `runs/2026-10-20-05/perf.md`.

- **Verdicts:** **Function: GREAT** — one key crosses; the project is never re-read (2 reads before,
  2 after three crossings); the teardown is still a teardown. **Zoom: GREAT** — the cue is one
  statement in one place, the key is legible, and the hierarchy puts the panels first and the cue
  beside them. **Fresh eyes: GREAT** — the cue is on screen without being pressed, the key is one
  `K` away, and the map comes back as the map you left. All three in
  `runs/2026-10-20-05/review.md`, with the captures in `runs/2026-10-20-05/evidence/`.

- **Commit:** `a3b9013` - "bevy_emerge_mapper: one key crosses the doors, and the project comes
  with it". Pushed to `Ladvien/bevy_emerge` `main`.
  **No PR**, for the same reason as the previous four runs: this repository has never had one,
  every commit goes straight to `main`, and `gh pr create` refuses with *"head branch `main` is
  the same as base branch `main`"*. The commit was already on `main` when the PR step was
  reached, and the owner's rule of 2026-10-20 says a commit that landed on the wrong branch is
  *"fixed with a new commit or a question, never by resetting a shared working tree or
  force-pushing"*. The branch `feature/stamps-reach-the-consumer` from the run before is still
  on the remote at the same commit; **deleting it is left to the owner**.

- **Run:** `docs/loop/runs/2026-10-20-05/` — `research.md`, `plan.md`, `perf.md`, `review.md`,
  `evidence/`.

Top 5 remaining:

1. **The six missing editor operations** — kit rename, project rename, project delete, bash
   creation, policy patch add, composition rename/delete. A scout of `bevy_emerge_core` found **no
   function for any of the six** — not even one without a UI caller. Each needs a new on-disk writer
   with its own re-parse-before-save discipline, and the scout's note is that a `Patch` has a
   multi-line `because` and a nested `Descriptor`, so `LineDoc::append` — the one-line-record tool —
   is not a direct fit. **The door switch is what makes them all easier**: an author renames a kit
   on the Kit door and then wants to see the map that uses it.
2. **The 282 hand-placed fences.** Measured, and the largest single number in the map. But
   `rules.ron` already says the *game* lays the blocks out, and the fences are the *border* of a
   block — so this is content, and the last three iterations called it the owner's.
3. **`fill.rs` has no test at all** — 1,058 lines, the one untested module in the mapper, and the
   module that lays the bulk of a level. A flood and a box fill are the two gestures that make a
   level cheap, and nothing pins their lattice, their refusals, or their idempotence.
   *(**Corrected 2026-10-20-07b: the module has 20 inline tests, and two of them bracket the
   defect exactly — one says a fill truncates and says so, the other says a fill starting on
   something is refused — and neither asks what the **next** fill does. The claim was stale.)*
4. **`vlm.rs:1436-1442`** — `probe` returns `Ready` for any non-loopback host **without
   connecting**, so a LAN endpoint that is down is reported ready. Needs a test that calls `probe`,
   not `is_near`.
5. **The composition *authoring* path.** The COMPOSE tab reads a group; it does not author one.
   `compositions.ron` is hand-written today — sixteen houses, each with 18–38 members, written by
   hand — and a group captured on the map (`M`) becomes a composition. **The capture exists and the
   authoring does not**, and the two halves of that are the same shape as the door split: the tool
   for making one and the tool for editing one are on different doors, and now they are one key
   apart.

## 2026-10-20-06: the composition can be renamed — and every map that stamps it follows

- **Change:** the COMPOSE tab can now **rename the composition it is showing** — `I` opens the name,
  `Enter` keeps it, `Esc` leaves it alone — and **every map that stamps the composition follows**.
  Two changes: the core grows one writer, `composition::rename_composition`, which edits
  `compositions.ron` and every map's stamps **without reformatting either file**; and the COMPOSE
  tab grows the verb, the same shape as the Meshes tab's `I` field, with the same refusals.

  **The editor could read a group in rich detail and write a new one, and could not change a
  group's *name*** — which is the one edit every author needs, and the one the file's own format
  makes trivial (**one line**).

  **The name is forced as it is typed**, and **the namespace survives** — a composition's id is
  per-segment (`town/house_cottage_plaster` is *which kit* and *which piece*), so the forcing is
  `naming::to_id`, a function this run added to the core.

- **Why it ranked first:** it is **one of the six missing editor operations** the last review
  listed, and the only one the editor's own tab is *about*. Measured on Sanctuary's project:
  `compositions.ron` is **5,817 lines and 204 KB**, and it is **hand-written**, because until this
  run the editor could not write it. The editor's own capture makes new ones — and the write
  re-serialized the whole file, dropping every comment.

  **The runner-up is renaming a kit's namespace: 744 mentions across 4 files, and no multi-file
  atomicity** — a half-done rename **leaves a project that will not open**. This one is bounded:
  one line in one file, plus the references the editor can enumerate because it has the whole
  project in memory.

- **Sources:** Feldthaus & Møller, *Semi-automatic rename refactoring for JavaScript* (2013),
  [10.1145/2544173.2509520](https://doi.org/10.1145/2544173.2509520) — *"there is often no
  practical alternative to simple syntactic search-and-replace"*, and the answer is *"a
  combination of static analysis and interaction with the programmer"*. Overbey, *A toolkit for
  constructing refactoring engines* (2012) — the **precondition check**, whose three properties
  this project already answers: `naming::is_id` (input validity), `composition::validate`
  (compilability), the fingerprint (preservation). Kappel & Schrefl, *Local referential integrity*
  (1992) and Wei, Leck & Link (ICDE 2019),
  [10.1109/ICDE.2019.00175](https://doi.org/10.1109/ICDE.2019.00175) — **a rename is an operation
  that must maintain every reference, atomically**. Full notes in `runs/2026-10-20-06/research.md`.

- **The fingerprint is not in the way, and the measurement says so.** A composition's own id *is*
  folded into its fingerprint (`f.str(&comp.id)`), so a renamed composition is a different
  composition *to the fingerprint* — **and nothing compares against that fingerprint.**
  `Stamped::of_fingerprint` is *"what the composition looked like when this was stamped"* and **no
  code writes it**; it is `None` on every stamp in the shipped file, and `None` means *"never
  recorded"*, which is a different fact from stale. The `Stale` badge reads `Member::of_fingerprint`,
  which records the **descriptor's** fingerprint. **So the rename's whole job is the name and the
  reference** — the members, the envelope and the notes are untouched.

- **The one writer, and the crate says where it lives.** `tests/compose_is_read_only.rs` holds the
  line that *"`project.rs` is the only module that names the file, and every tab reaches it
  there."* The first draft put the writer in `compose.rs` and **failed that test** — the crate's
  own statement of the rule caught the design, and the writer moved to `project.rs` as
  `Project::rename_composition`.

- **Tests:**
  - `crates/bevy_emerge_core/tests/a_composition_can_be_renamed.rs` — **9 tests**: the file is
    edited and not reformatted (a hand-written note survives byte for byte), every map that stamps
    the composition follows, a map that does not is **untouched byte for byte**, a taken name is
    refused, an illegal id is refused, a missing composition is refused, the members are untouched,
    the rename is **reversible**, and the renamed file still parses.
  - `crates/bevy_emerge_mapper/tests/a_composition_can_be_renamed.rs` — **5 tests**: the door does
    the work and the map follows, a collision is refused and the file is byte-identical, the tab
    **reaches the door** rather than writing itself, the namespace survives the field, and the
    field holds the keyboard while it is open.
  - `crates/bevy_emerge_core/src/naming.rs` — **4 new tests** for `to_id`.
  - `cargo test --workspace` → **1,252 pass, 1 fail** — the one failure is
    `bevy_emerge_mapper --test headless`'s `no_two_leaders_cross`, **red since at least
    2026-09-03** and unchanged by this run.
  - Clippy: **zero findings in every file this run touches**; the 15 in `composition.rs` are the
    same 15 as the baseline, shifted by the 200 lines this run added above the test module.
  - `cargo fmt --check`: **no new diff in the files this run touches** — `compose.rs` 94,
    `editor.rs` 87, `keys.rs` 22, `project.rs` 11, `chrome.rs` 16, **all the baseline's counts.**
    (A blanket `rustfmt` was tried and **reverted**: it turned a +282-line change into a
    +734-line one by reformatting 67 hunks of code this run never touched.)
  - `cargo doc --no-deps`: no new warnings; the ones present are pre-existing and in files this run
    did not touch.
  - Headless boot against Sanctuary's real project: **12 frames, 4,096 entities, no panic.**
  - Sanctuary's own suite: **1,018 pass, 4 fail** — and **the same four with the emerge change
    stashed** (`a_prop_on_a_host_rests_on_its_top`, `the_saved_version_is_five_and_a_save_without_a_
    forest_section_reads_with_an_empty_forest`, `a_flanker_walks_its_route_at_its_speed_and_is_gone_
    and_counted_at_the_end`, `the_board_is_three_trees_and_every_item_has_its_effect_written`),
    **verified by running the suite with this run's change stashed.** The game has **no callsite**
    in `naming` or the composition writer, and it discovers compositions by **prefix**
    (`HOUSE_PREFIX = "town/house_"`), so a rename within that namespace strands nothing.
  - Timings in `runs/2026-10-20-06/perf.md`.

- **The visual review found two real defects, and both are fixed.** Recorded here because the
  briefs and the tests did not catch them:
  1. **The namespace was being dropped.** The field forced the name with `naming::to_snake_case`,
     which drops every character that is not a letter or a digit — **including the `/`**. So
     `town/house_cottage_brick` came out as `town_house_cottage_brick`: a different id, in a
     different namespace, **colliding with nothing**, so no refusal was owed and the field
     **showed one spelling and wrote another**. The rule was already in the crate — `is_id` reads
     the namespace — and **the forcing was not**, so `naming::to_id` was added: per segment, with
     the separators kept.
  2. **The receipt never survived.** `Action::ComposeArm` is `Enter` on the COMPOSE tab, and the
     rename's field is committed with `Enter` too — so the same keystroke **named the composition
     and armed it**, and the arm's message overwrote the rename's receipt. `Phase::Text` runs
     *before* `Phase::Act`, so the phases do not save it. **The crate already had the answer**, in
     `editor::Fields` — *"Adding a field is adding a line, here and nowhere else"* — and the
     COMPOSE tab's field was simply not in that list. **One line**, in the one place the crate
     says a field is added, and the bug is gone. A bespoke guard in `compose.rs` was written first
     and **removed**: the crate's mechanism is the right one.

- **Verdicts:** **Function: GREAT** — every state captured, and the refusal names both ids and
  writes nothing. **Zoom: GREAT** — the field's forced spelling is legible at 2x, the namespace
  slash is there, the receipt wraps sensibly. **Fresh eyes: GREAT** — the key overlay says
  **`I` — rename this composition**, the field opens empty, the receipt says *"Rename it back to
  undo"*, and the round trip leaves `compositions.ron` **byte-identical** to Sanctuary's original.
  All three in `runs/2026-10-20-06/review.md`, with the captures in
  `runs/2026-10-20-06/evidence/`.

- **Measured, on Sanctuary's own project:** the rename changes **one word in two files** —
  `compositions.ron` stays at **5,817 lines**, `town.map.ron` stays at **751 lines**, the
  hand-written `note:` is **intact byte for byte**, and the map that does not stamp the composition
  is **not written at all**.

- **Commit:** `9690dfe` - "bevy_emerge_core, bevy_emerge_mapper: the composition can be renamed".
  Pushed to `Ladvien/bevy_emerge` `main`.
  **No PR**, for the same reason as the previous five runs: this repository has never had one
  (`gh api repos/Ladvien/bevy_emerge/pulls` returns **0**), every commit goes straight to `main`,
  and `gh pr create` refuses with *"head branch `main` is the same as base branch `main`"*. The
  commit was already on `main` when the PR step was reached, and the owner's rule of 2026-10-20
  says a commit that landed on the wrong branch is *"fixed with a new commit or a question, never by
  resetting a shared working tree or force-pushing"*. The branch `feature/stamps-reach-the-consumer`
  from the run before that is still on the remote at the same commit; **deleting it is left to the
  owner**.

- **Run:** `docs/loop/runs/2026-10-20-06/` — `research.md`, `plan.md`, `perf.md`, `review.md`,
  `evidence/`.

Top 5 remaining:

1. **Renaming a kit's namespace** — the same idea as this run, with a much larger blast radius.
   Measured over Sanctuary's project: `maps/town.map.ron` **383** mentions of `town/`,
   `maps/town_block.map.ron` **289**, `town/library.ron` **49**, `compositions.ron` **16**,
   `rules.ron` **7** — **744 mentions across 4 files**. And there is **no multi-file atomic
   primitive**: `save_atomic` is per file, and a half-finished rename leaves a project that **fails
   to open**, because `bound_library` refuses a namespace mismatch whichever file is written first.
   **This one wants the atomicity designed before the verb**, and it is now the only one of the six
   missing operations whose *shape* this run has already shown: a writer that returns **every** new
   text, and a caller that writes them **only if every one succeeded**.
2. **The other five missing editor operations** — project rename, project delete, bash creation,
   policy patch add, composition delete. The policy patch is the hardest: a `Patch` has a
   multi-line `because` and a nested `Descriptor`, so `LineDoc::append` — the one-line-record tool
   — is not a direct fit. **Composition delete is this run's writer with the opposite direction**,
   and the maps that stamp it would have to lose their stamps, which is a *deletion* decision
   rather than a rename.
3. **The 282 hand-placed fences.** Measured, and the largest single number in the map. But
   `rules.ron` already says the *game* lays the blocks out, and the fences are the *border* of a
   block — so this is content, and the last three iterations called it the owner's.
4. **`fill.rs` has no test at all** — 1,058 lines, the one untested module in the mapper, and the
   module that lays the bulk of a level. A flood and a box fill are the two gestures that make a
   level cheap, and nothing pins their lattice, their refusals, or their idempotence.
   *(**Corrected 2026-10-20-07b: the module has 20 inline tests, and two of them bracket the
   defect exactly — one says a fill truncates and says so, the other says a fill starting on
   something is refused — and neither asks what the **next** fill does. The claim was stale.)*
5. **`vlm.rs:1436-1442`** — `probe` returns `Ready` for any non-loopback host **without
   connecting**, so a LAN endpoint that is down is reported ready. Needs a test that calls `probe`,
   not `is_near`.

## 2026-10-20-07: the editor's story about a named thing matches the disk

- **Change:** a map can now be **renamed safely**, and the editor's account of a named thing and the
  disk's account of it no longer come apart. Four defects, all measured, all fixed:
  1. **A rename onto a name another map has destroyed that map.** `OpenMap::save` derived the path
     from the map's *name*, and `ron_surgery::save_atomic` ended in `fs::rename` — which POSIX
     specifies as **replacing** the target (IEEE Std 1003.1-2024: *"if the directory entry named by
     `new` exists, it shall be removed and `old` renamed to `new`"*). **Now the refusal is the
     kernel's**: `ron_surgery::save_atomic_no_replace` uses `link(2)`, which fails with `EEXIST`
     rather than replacing, and never modifies the target.
  2. **A rename left a duplicate.** The old file was never removed, so the project ended up with
     **two files answering to one map**, and the stale one refuses to open with a message about a
     schema mismatch. **The old file is now removed by the save** — after the new one lands — and a
     failed removal is *reported*, not discarded.
  3. **One `Escape` too few left the chooser's MAP INFO panel describing a map that does not
     exist.** The name field's `Escape` sets `swallowed`, which stops the chord handler — the only
     other place that clears a draft — from reading the same press, so a `New::Map` draft survived
     and `map_rows`' `(Some(New::Map(d)), _)` arm kept winning over the selected map. **The panel
     read "NEW MAP IN kit / NAME (needs a name) / BOUNDS 10 x 4 x 10" while the MAPS list beside it
     highlighted a real map with 727 pieces.** The field's `Escape` now clears the draft it belongs
     to.
  4. **Clicking a settings row's value opened the wrong field.** The click moved the focus but not
     the field, so clicking `NAME` opened `ORIGIN` and the author's map name went into the origin:
     *"0 x 0 x 0town_block_ is not three numbers"*. The `•` mark — drawn from the same value the
     field handler reads — now lands on the row that was clicked.

  **And a fifth, smaller one:** the KIT INFO panel's row `new work lands here` / `yes` rendered with
  a **5 px** gap, so it read as one phrase, while the rows above had **133 px**. `chrome::row_label`
  uses `min_width` (a floor) and the row's `column_gap` is a **flex** gap, so a label *wider* than
  its column pushes its own box wider and swallows the gap. **The label now carries a margin of its
  own**, and every panel that holds a phrase is fixed at once.

- **Why it ranked first:** these defects **happen on every map, on every rename, and one of them
  destroys work.** The brief's phrase for "most usable" is *"an author gets a real project built
  faster, with fewer sharp edges, and **trusts the result**"*, and a tool that can destroy a map
  without saying so, or show a panel about the wrong map, is not trusted — **and every other feature
  the editor has inherits that.** The runner-up, **renaming a kit's namespace**, is 744 mentions
  across 4 files with no multi-file atomicity — a **larger** feature, but a *rarer* one: it happens
  **once per project**, and the load-time refusals already stop a half-done rename from opening.
  **A feature that fires once a project is not worth a day; a feature that fires on every save is.**

- **Sources:** POSIX `rename()` ([IEEE Std 1003.1-2024](https://pubs.opengroup.org/onlinepubs/9799919799/functions/rename.html))
  — *"if the directory entry named by `new` exists, it shall be removed"* — and the no-replace
  primitive that fixes it (macOS's `renamex_np(RENAME_EXCL)`, `EEXIST`; Linux's
  `renameat2(RENAME_NOREPLACE)`). Lampson, *Hints for computer system design* (SOSP 1983,
  [10.1145/800217.806614](https://doi.org/10.1145/800217.806614)) — *"Make actions atomic or
  restartable."* Schlichting & Schneider, *Fail-stop processors* (ACM TOCS 1983) — *"halts in response
  to any internal failure and does so before the effects of that failure become visible."* Pillai et
  al., *All file systems are not created equal* (OSDI 2014,
  [10.5555/2685048.2685082](https://doi.org/10.5555/2685048.2685082)) — on why a two-step update
  protocol is where correctness is lost. Hutchins, Hollan & Norman, *Direct Manipulation Interfaces*
  (HCI 1985, [10.1207/s15327051hci0104_2](https://doi.org/10.1207/s15327051hci0104_2)) — the **gulf
  of evaluation**, which is a panel describing the wrong object. Norman, *Design rules based on
  analyses of human error* (CACM 1983, [10.1145/2163.358092](https://doi.org/10.1145/2163.358092)) —
  the **slip** classification, which is exactly what a stale draft after `Escape` is. Shneiderman,
  *Direct Manipulation* (IEEE Computer 1983,
  [10.1109/MC.1983.1654471](https://doi.org/10.1109/MC.1983.1654471)) — *"Permit easy reversal of
  actions… the user knows that errors can be undone."* Full notes in
  `runs/2026-10-20-07/research.md`.

- **Why `link(2)` and not `renameat2`/`renamex_np`.** `renameat2(RENAME_NOREPLACE)` is the exact
  primitive and is the right one **where it exists** — but it is Linux-only, and the editor runs on
  macOS, where the equivalent is a libc call with an `unsafe` declaration. **This crate has no
  `unsafe` today and no `libc` dependency.** `link(2)` is POSIX, is on every filesystem this crate
  runs on, **creates the target as a second name for the same inode, and fails with `EEXIST` if the
  target exists** — and it never modifies the target. **One call, so the refusal is atomic with the
  write.** A check-then-write would not do: POSIX gives atomicity to the single call, so the target
  appearing between the check and the write would be destroyed. `EXDEV` is a **refusal, not a
  fallback** — a fallback is the defect the function exists to remove.

- **Tests:**
  - `crates/bevy_emerge_core/tests/a_rename_never_replaces.rs` — **7 tests**: the write lands; a
    taken path is refused and the target's bytes are unchanged; **a file that appears before the
    write is still refused**; the refusal names the path; no `.tmp` is left on either path; the
    parent directory is made.
  - `crates/bevy_emerge_mapper/tests/a_rename_is_all_or_nothing.rs` — **5 tests**: a rename onto a
    taken name is refused and the other map's bytes are unchanged; a rename leaves **one** file,
    named for the map, whose own `name:` agrees; a rename and back leaves the same set of files;
    the name field's `Escape` clears the draft; a settings click sets the field.
  - `crates/bevy_emerge_mapper/tests/a_label_and_its_value_are_two_things.rs` — **3 tests**: the
    label carries a margin of its own; the margin is the design system's own `GAP_ROW`; the long
    label is wider than its column.
  - **Every test was checked against a build without its fix** — and one **failed that check and
    was rewritten**: the first gap test compared two *measured* gaps and **passed with the fix
    removed**, because the **headless layout does not reproduce the defect**. It now asserts the
    mechanism, and the measurement moved to the visual review, where the window is.
  - `cargo test --workspace` → **1,267 pass, 1 fail** — the one failure is
    `bevy_emerge_mapper --test headless`'s `no_two_leaders_cross`, **red since at least
    2026-09-03** and unchanged by this run.
  - Clippy: **one finding, in `grammar.rs`** — a file this run does not touch.
  - `cargo fmt --check`: **no new diff in the files this run touches** — `ron_surgery.rs` 5,
    `project.rs` 11, `chooser.rs` 78, `chrome.rs` 16, `editor.rs` 87, **all the baseline's counts.**
  - `cargo doc --no-deps`: no new warnings.
  - Headless boot against Sanctuary's real project: **12 frames, 4,096 entities, no panic.**
  - Sanctuary's own suite: **1,018 pass, 4 fail** — **the same four with this change stashed**
    (`town_map`, `save`, `flankers`, `research`).
  - Timings in `runs/2026-10-20-07/perf.md`.

- **Measured, on a copy of Sanctuary's project:** a rename to a fresh name moved
  `town_block.map.ron` (77,947 bytes) to **`town_renamed.map.ron`** and **removed the old file** —
  one map, one file, one name, and the file's own `name:` agrees with its filename. A rename onto a
  taken name changed **nothing**: both files byte-identical afterwards.

- **The consumer's callsites: none broken.** The change is inside `bevy_emerge_mapper` and
  `bevy_emerge_core::ron_surgery`; the game calls neither, and it reads maps by name from a list it
  owns. **But a rename strands the game**, because a map's name is **a string in Sanctuary's code**
  — `town_map.rs:35` `MAP = "town"`, `emerge_data.rs:28` `MAPS = ["town_block", "town"]` — and the
  editor's receipt now says so. **This is the opposite of a composition rename**, which the game
  finds by *prefix* and which strands nothing; both were checked, not assumed.

- **Verdicts:** **Function: GREAT** — every state captured, the refusal and the move both measured
  on disk. **Zoom: GREAT** — the gap fix measured on a controlled before-and-after (19 px → 29 px
  for the long label, 131 px → 143 px for the short one), and the `•` mark on the row that was
  clicked. **Fresh eyes: GREAT** — the key overlay names the field, the field opens holding what is
  in it, the refusal writes nothing, and the move leaves one file. All three in
  `runs/2026-10-20-07/review.md`, with the captures in `runs/2026-10-20-07/evidence/`.

- **An incident, recorded because it is part of this run's evidence:** the consumer's
  `AGENTS_CHAT.md` was **overwritten instead of appended to**, and **1,095 lines of other agents'
  notes were lost**. The file is gitignored, so git had no copy, and the session transcripts hold
  only 12 lines of it. The recoverable 12 are restored, **the loss is stated in the file itself**,
  and the whole account is in `runs/2026-10-20-07/an-incident.md`. **The lesson, and the rule:**
  `write` replaces, so a file that says to append must be appended to — and a file another agent
  owns is a message, and a message is not replaced.

- **Commit:** `303adc0` - "bevy_emerge_core, bevy_emerge_mapper: a map's name IS its file, and a
  rename is a move". Pushed to `Ladvien/bevy_emerge` `main`.
  **No PR**, for the same reason as the previous six runs: this repository has never had one
  (`gh api repos/Ladvien/bevy_emerge/pulls` returns **0**), every commit goes straight to `main`,
  and `gh pr create` refuses with *"head branch `main` is the same as base branch `main`"*. The
  commit was already on `main` when the PR step was reached, and the owner's rule of 2026-10-20
  says a commit that landed on the wrong branch is *"fixed with a new commit or a question, never by
  resetting a shared working tree or force-pushing"*. The branch `feature/stamps-reach-the-consumer`
  from an earlier run is still on the remote at the same commit; **deleting it is left to the
  owner**.

- **Run:** `docs/loop/runs/2026-10-20-07/` — `research.md`, `plan.md`, `perf.md`, `review.md`,
  `an-incident.md`, `evidence/`.

Top 5 remaining:

1. **Renaming a kit's namespace** — the same idea with a much larger blast radius: **744 mentions
   across 4 files** (`town.map.ron` 383, `town_block.map.ron` 289, `town/library.ron` 49,
   `compositions.ron` 16, `rules.ron` 7). **And there is no multi-file atomicity**: `save_atomic` is
   per file, and a half-finished rename leaves a project that **fails to open**, because
   `bound_library` refuses a namespace mismatch whichever file is written first. **This run's
   `save_atomic_no_replace` is one piece of that atomicity** — the piece that refuses instead of
   replacing — and the shape is the same: a writer that returns **every** new text, and a caller
   that writes them **only if every one succeeded**.
2. **The other four missing editor operations** — project rename, project delete, bash creation,
   policy patch add. The policy patch is the hardest: a `Patch` has a multi-line `because` and a
   nested `Descriptor`, so `LineDoc::append` — the one-line-record tool — is not a direct fit. The
   scout's measurement: Sanctuary's `kit/project.ron` carries **18 hand-written patch records** with
   the same `because` text, and the append primitive and the remove both exist — **only the add path
   has no caller.**
3. **The 282 hand-placed fences.** Measured, and the largest single number in the map. But
   `rules.ron` already says the *game* lays the blocks out, and the fences are the *border* of a
   block — so this is content, and the last three iterations called it the owner's.
4. **`fill.rs` has no test at all** — 1,058 lines, the one untested module in the mapper, and the
   module that lays the bulk of a level. A flood and a box fill are the two gestures that make a
   level cheap, and nothing pins their lattice, their refusals, or their idempotence.
   *(**Corrected 2026-10-20-07b: the module has 20 inline tests, and two of them bracket the
   defect exactly — one says a fill truncates and says so, the other says a fill starting on
   something is refused — and neither asks what the **next** fill does. The claim was stale.)*
5. **`vlm.rs:1436-1442`** — `probe` returns `Ready` for any non-loopback host **without
   connecting**, so a LAN endpoint that is down is reported ready. Needs a test that calls `probe`,
   not `is_near`.

## 2026-10-20-07b: the editor's refusals reach the author, and no message promises what the tool cannot do

- **Change:** **a fill that cannot be done is refused where the author can see it, and a fill that
  stops at the cap says what to do next.** Four changes, all one defect seen four ways — *the
  editor's account of what just happened and the truth about it come apart*:
  1. **A refusal reaches the author.** The editor's `Status` has two slots and its own doc says
     which belongs where; **the fill's three refusals were in the wrong one** — the receipt slot,
     which is one dim row, erased by the next receipt and by a tab switch. **They are now
     `Status::problem`s**, which reach the toast and the journal.
  2. **No message promises what the tool cannot do.** The flood's cap said *"fill again to
     continue"*, and **the second fill cannot continue** — the flood will not start on a cell that
     already holds a piece, and the cell the author clicks is the cell the first fill filled.
  3. **The cap's message names a route that works** — a fill from another empty cell, which
     **places 4,096 more** because the flood stops at anything already placed. Measured.
  4. **The two fills say the same kind of thing.** The box fill said *"stopped at the 4096 cell
     cap"* — true; the flood said something else, and **it was the one on the more-used gesture**.

- **Why it ranked first:** these defects **happen on every refusal in the editor** — a survey
  found **21 of them in the wrong slot** — and **one of them destroys work**. The brief's phrase for
  "most usable" is *"an author gets a real project built faster, with fewer sharp edges, and
  **trusts the result**"*, and a tool that says *"fill again to continue"* and then refuses the
  second fill, **and says nothing the author can see**, is not trusted — **and every other feature
  the editor has inherits that.** The runner-up, **renaming a kit's namespace**, is a *larger*
  feature but a *rarer* one: it happens **once per project**, and the load-time refusals already
  stop a half-done rename from opening. **A feature that fires once a project is not worth a day; a
  feature that fires on every refusal is.**

- **Sources:** NN/g, *Error-Message Guidelines*
  ([nngroup.com](https://www.nngroup.com/articles/error-message-guidelines/)) — *"The very worst
  error messages are those that don't exist. When users make a mistake and receive no feedback, it
  can create a cascade of misunderstanding, wasted effort, and frustration"*; *"Merely stating the
  problem is also not enough; offer some potential remedies"*; *"Design errors based on their
  impact"*. Myers, *The Importance of Percent-Done Progress Indicators* (CHI 1985,
  [10.1145/317456.317459](https://doi.org/10.1145/317456.317459)) — and its finding is a **caution
  rather than a support**: the bar *"did not need to be accurate"*, so a progress indicator is
  judged on its presence and its honesty about being **finished**. Google AIP-158 (Pagination) —
  the primary standard for a resumable cursor: *"Page tokens **must** be limited to providing an
  indication of where to continue"*, and *"If the end of the collection has been reached… the
  `next_page_token` field **must** be empty. This is the *only* way to communicate 'end-of-collection'
  to users."* Zilberstein, *Using Anytime Algorithms in Intelligent Systems* (AI Magazine 1996,
  [10.1609/aimag.v17i3.1232](https://doi.org/10.1609/aimag.v17i3.1232)) and Hansen & Zilberstein,
  *Monitoring Anytime Algorithms* (ACM 1996,
  [10.1145/242587.242593](https://doi.org/10.1145/242587.242593)) — an anytime computation keeps a
  usable answer when interrupted, and **the fill's "fill again to continue" was an anytime contract
  it did not honour**. Stanshine, *Modeling silent failures in telecommunications systems* (RAMS
  1995, [10.1109/rams.1995.513255](https://doi.org/10.1109/rams.1995.513255)) — *"A silent failure
  is a condition which… remains undetected by normal fault detection methods"*, which is what a
  quiet refusal is. Norman, *Design rules based on analyses of human error* (CACM 1983,
  [10.1145/2163.358092](https://doi.org/10.1145/2163.358092)) — the **slip** classification: the
  author who clicks "fill again" has a proper intention, and the editor does something else.
  Hutchins, Hollan & Norman, *Direct Manipulation Interfaces* (HCI 1985,
  [10.1207/s15327051hci0104_2](https://doi.org/10.1207/s15327051hci0104_2)) — the **gulf of
  evaluation**. Full notes in `runs/2026-10-20-07b/research.md`.

- **The measurement, and it is the heart of the run.** The flood's cap message said *"fill again to
  continue"*, and:

  | what the author does | what happens | measured |
  |---|---|---|
  | **fills again in the same cell** | **refused** — *"there is already something here"* | the flood will not start on a cell that already holds a piece |
  | **fills again in a *different* empty cell** | **works** — 4,096 more | the flood stops at anything already placed, and the first fill's pieces are anything already placed |

  **So the promise was achievable, but only by clicking somewhere new — and nothing said so.** And
  the refusal that followed **was not on screen**: the INSPECTOR's receipt row was empty and the
  journal said *"nothing has gone wrong yet"*, while the **ghost's** refusal for the same piece was
  plainly visible in the amber row, because the ghost writes a different field.

- **The `Status` doc was right and the code was wrong.** `Status::note`'s own doc says it is for
  *"a refusal the author is not owed an alarm about — clicking bare ground with the remove tool
  armed… **Those are answers, not failures**"*, and `Status::problem`'s says it is for *"the editor
  could not do what was asked, **or did only part of it**"*. **The fill's three sites had comments
  saying the opposite** — *"A refusal is the answer, not a failure"* — because they read the rule
  from the wrong half of the contract. **The code had been quoting itself.** The doc now draws the
  line, and says which three sites crossed it.

- **The slice, and why it is three of the twenty-one.** A survey of every `status.note` call site
  in the mapper found **21 refusals in the wrong slot** — in the fill, the placement, the drag, the
  rename, the labeller and the tile operations. **Each is one line, and each is the same line.**
  **This run takes the fill's three**, because the two that matter — the cap's false promise and
  the invisible refusal — are both there, and the fill is **the gesture that lays the bulk of a
  level**. **And the reason for not taking all twenty-one at once is the habituation literature**
  (*Tuning Out Security Warnings*, MISQ 2018,
  [10.25300/misq/2018/14124](https://doi.org/10.25300/misq/2018/14124)): twenty-one new red
  blocks at once is exactly the habituation it warns about. **The other eighteen are the next
  item, and the log says where they are.**

- **Tests:**
  - `crates/bevy_emerge_mapper/src/fill.rs` — **2 new tests**: *the cap's promise cannot be kept by
    a second fill*, and *a flood from a new cell carries on*. **The first is the proof**: it asserts
    a fact about the **algorithm**, and it passes with the fix in place — because the **message**
    was the thing that was wrong, and the test pins the truth the message now tells.
  - `crates/bevy_emerge_mapper/tests/a_fill_tells_the_truth.rs` — **4 tests**: a refused fill is a
    `problem`; a refusal survives a receipt and a tab switch; the cap's message names a route that
    works and does not promise the one that does not; the two fills say the same kind of thing.
  - `crates/bevy_emerge_mapper/tests/a_refusal_reaches_the_author.rs` — **5 tests**: the three fill
    sites are `problem`s; a refusal survives; the cap's message; the two fills; a stopped fill and
    a finished fill differ.
  - `crates/bevy_emerge_mapper/tests/the_refusal_is_on_screen.rs` — **1 test**: the refusal is on a
    node with a real size.
  - `crates/bevy_emerge_mapper/tests/the_toast_is_on_screen.rs` — **1 test**: the refusal is a
    **card**, in the viewport, where the author is looking.
  - **Every test was checked against a build without its fix**, and **five of them fail without it**
    — including the two that measure the layout, which is what settled the question the captures
    could not: the toast is on for **seven seconds** and is a centred card in the viewport, so a
    capture taken at the wrong moment shows nothing and looks like a failure.
  - `cargo test --workspace` → **1,279 pass, 1 fail** — the one failure is
    `bevy_emerge_mapper --test headless`'s `no_two_leaders_cross`, **red since at least
    2026-09-03** and unchanged by this run.
  - Clippy: **no new finding in any file this run touches.**
  - `cargo fmt --check`: **no new diff in the files this run touches** — `editor.rs` 87,
    `chrome.rs` 16, `fill.rs` 0, **all the baseline's counts.**
  - Headless boot against Sanctuary's real project: **12 frames, 4,096 entities, no panic.**
  - Timings in `runs/2026-10-20-07b/perf.md`.

- **A flake this run's tests made likely, and fixed.** `chooser::tests::a_new_project_opens_and_
  bootstraps` began failing in the full-suite run. **It is a pre-existing race on a shared path**:
  `create_project` makes a *sibling* of the directory it is given, so the test created the project
  at `<temp>/porch_kit` — **a fixed path in the directory every test in every binary shares** —
  and the line that removed *"a sibling the previous run left behind"* removed the directory
  **another run was writing**. **Three more test files meant more parallel workers, and the race
  became reachable.** The project is now made **inside the test's own directory**, so nothing else
  can be inside it and nothing else can delete it. **The project's name and the assertion are
  unchanged.** Five clean runs of the mapper's lib suite after the fix.

- **A stale claim in this log, corrected.** The list said *"`fill.rs` has no test at all — 1,058
  lines, the one untested module in the mapper"*. **It has 20 inline tests**, and this run adds
  two more. **The claim was stale**, and the module's own tests **bracketed the defect exactly** —
  one says a fill truncates and says so, the other says a fill starting on something is refused —
  **and neither asked what the *next* fill does.** That is the question between them, and it is
  the author's only route past the cap.

- **Verdicts:** **Function: GREAT** — the three fill refusals reach the toast and the journal; the
  cap names a route that works; both fills say the same thing. **Zoom: GREAT** — the measurement
  moved to the layout, where the claim lives: the refusal is a card, in the viewport, surviving a
  receipt and a tab switch, and the cap's message at 2× names the way on. **Fresh eyes: GREAT** —
  the key overlay names the verb, the ghost warns before the click, the refusal is a card and a
  journal line, and the cap says what to do. All three in `runs/2026-10-20-07b/review.md`, with the
  captures in `runs/2026-10-20-07b/evidence/`.

- **Commit:** `2be95bb` - "bevy_emerge_mapper: the editor's refusals reach the author, and no
  message promises what the tool cannot do". Pushed to `Ladvien/bevy_emerge` `main`.
  **No PR**, for the same reason as the previous seven runs: this repository has never had one
  (`gh api repos/Ladvien/bevy_emerge/pulls` returns **0**), every commit goes straight to `main`,
  and `gh pr create` refuses with *"head branch `main` is the same as base branch `main`"*. The
  commit was already on `main` when the PR step was reached, and the owner's rule of 2026-10-20 says
  a commit that landed on the wrong branch is *"fixed with a new commit or a question, never by
  resetting a shared working tree or force-pushing"*. The branch `feature/stamps-reach-the-consumer`
  from an earlier run is still on the remote at the same commit; **deleting it is left to the
  owner**.

- **Run:** `docs/loop/runs/2026-10-20-07b/` — `research.md`, `plan.md`, `perf.md`, `review.md`,
  `evidence/`.

Top 5 remaining:

1. **The other eighteen mis-slotted refusals** — measured, and each is one line: the placement
   (`editor.rs:4981`, `5017`), the drag move (`8156`), the subject resolution (`7832`), the
   labeller's three (`labels.rs:412`, `464`, `988`), the composition rename's two
   (`compose.rs:986`, `1003`), the tile operations (`tiles.rs:2560`, `1718`), the build's two
   (`build.rs:1718`, `1795`), and the input refusals (`editor.rs:3091`, `5127`, `5685`, `8986`).
   **The mechanism is now proven** — the fill's three are the proof — **and the habituation
   literature says the batch comes after the first three have been seen in place.**
2. **A resumable flood with a real cursor** — AIP-158's shape, bound to the piece, the map and the
   region. **The flood's behaviour is already correct**; the *cursor* is the new mechanism, and it
   needs its own preconditions and its own refusals.
3. **Renaming a kit's namespace** — 744 mentions across 4 files, and no multi-file atomicity. A
   **larger** feature and a **rarer** one.
4. **`fill.rs` having no test** — **stale**, and corrected above.
5. **The 282 hand-placed fences** — content, and the owner's.

## 2026-10-21-01: the stage's labels, in one space

- **Change:** the **COMPOSE tab's stage labels** — the name under each group's miniature — are now
  placed in **the one space the drawing uses**, and they stop overlapping each other, stop being
  drawn **through their own miniature's wall**, and are withheld (visibly) when they do not fit.

- **The defect, in one sentence:** **the collision test ran in one space and the labels were drawn
  in another.** The boxes were measured from `world_to_viewport` (which answers in **physical**
  pixels) and from the node's `UiGlobalTransform` (the **window's** space, **physical**), while
  the drawing wrote the *node's* box into a `Val::Px` (which is **logical**). **So the test passed
  and the screen overlapped** — which is how the first version of this shipped.

- **Measured, on a live editor at `UiScale(2.4)`, Sanctuary's `town` kit, the focal group
  `town/house_cottage_plaster` (26 characters):**

  | | space | measured |
  |---|---|---|
  | `world_to_viewport` | **physical** px, relative to the camera's viewport rect | `(1765.0, 878.59)` |
  | `ComputedNode.size` | **physical** px | `412 × 32` |
  | `UiGlobalTransform.translation` | **physical** px, the node's **centre** | `(1885.0, 899.0)` |
  | `Val::Px(x)` | **logical** px — Bevy multiplies it by `UiScale` | — |

  **The check the numbers have to pass:** `Val::Px(699.58) × 2.4 = 1679`, and
  `1885 − 1679 = 206 = 412 / 2` — the node's centre, the written `Val::Px` and the node's own
  width agree, so **the scale is applied exactly once**.

  **And the advance constant was never wrong:** the focal node's 412 physical px over 26
  characters is **6.60** logical px per glyph, against `BODY_CHAR_W`'s **6.6**; the neighbour's 312
  over 24 is **5.42**, against `6.6 × 9/11 = 5.4`. **The constant was right; the space was not.**

- **The fix, in three parts:**
  1. **One space** — `compose::label_box` builds the box in **logical** pixels and the drawing
     writes it into the node **unchanged**, because `Val::Px` *is* logical pixels.
  2. **One anchor** — `compose::label_anchor` answers "where does this group's label go", and
     **both** the box builder and the drawing call it, so the test and the drawing cannot disagree
     about *where* the label is, only about whether it collides.
  3. **The right point** — the label goes under the footprint's **near ground edge**, not the
     group's origin. **And "lower" is the *larger* `y`** — a viewport's `y` grows downwards
     (`bevy_camera`'s `world_to_viewport_core` flips the NDC's `y` for exactly this reason), so the
     first version took the *smallest* `y` and put the label at the footprint's **far** edge,
     behind the miniature and therefore inside it.

- **Why it ranked first:** it is the only defect on the tab that **makes the editor lie about what
  it is showing** — three groups' names drawn on top of each other, and the group you are editing
  named *through its own house*. The runner-up (the `MESHES` tab's crossing badge leaders) is
  real and is **already a known baseline failure** (`no_two_leaders_cross`, red since 2026-09-03,
  in the log's own baseline), so it is carried rather than fixed.

- **Sources:** Bell, Feiner & Höllerer (2001), *View Management for Virtual and Augmented Reality*,
  UIST ([10.1145/502348.502363](https://doi.org/10.1145/502348.502363)) — *"If no such space can be
  found, then no label is allocated."* Azuma & Furmanski (2003), ISMAR
  ([10.1109/ismar.2003.1240689](https://doi.org/10.1109/ismar.2003.1240689)) — *"read labels
  fastest with the algorithms that most quickly prevented overlap."* Christensen, Marks & Shieber
  (1995), ACM TOG ([10.1145/212332.212334](https://doi.org/10.1145/212332.212334)) — the problem is
  NP-hard, so a fast always-correct rule beats an optimal one. Stein & Décoret (2008), NPAR
  ([10.1145/1377980.1377986](https://doi.org/10.1145/1377980.1377986)) — the order decides the
  loser, so it is stated. Furnas (2006), CHI
  ([10.1145/1124772.1124921](https://doi.org/10.1145/1124772.1124921)) — an elision must be
  **visible**.

- **Tests:** `cargo test --workspace` → **all green**. Three new test files:
  `the_stage_labels_live_in_one_space` (the conversion, measured against a known value — the scale
  applied **exactly once**), `the_stage_labels_are_on_screen` (the labels do not overlap,
  **measured in the real layout**, and there is one box builder and one anchor),
  `the_stage_labels_do_not_collide` (the deconfliction's rules).
  **The baseline's one failure, `no_two_leaders_cross`, is unchanged and confirmed pre-existing**
  (it fails identically on a stashed tree, and the log's own baseline records it red since
  2026-09-03).
  **`cargo fmt --check` and `clippy -D warnings` are pre-existing red across the workspace** (876
  fmt diffs at baseline, in files this run did not touch) — **this run added zero fmt diffs** and
  zero clippy findings of its own.

- **Perf:** five projections per label per frame instead of one (the four footprint corners plus
  the centre) — **no measurable cost**: the mapper's suite is 6.23 s against 6.02 s, inside the
  noise. And the change **removes** a second projection pass: the drawing used to recompute the
  projection for every label *in addition to* the box builder's pass.

- **Verdicts:** **Function NOT YET → GREAT** (the first capture had the focal label at the
  *left* of its house, because the anchor took the minimum in both axes), **Zoom GREAT** (the
  labels clear the miniatures; the amber/`DIM` hierarchy puts the eye on the group being edited),
  **Fresh eyes GREAT** (a cold start finds the labels where a label belongs, and the withheld
  neighbour reads as *"a group whose name is one action away"*).

- **Run:** `docs/loop/runs/2026-10-20-07c/` — `research.md`, `plan.md`, `perf.md`, `review.md`,
  `evidence/{before,after,crops}/`.

Top 5 remaining:
1. **A leader line for a withheld label.** The `…` marker is placed where the *name* would be,
   and the name is 24 characters wide, so a wide marker under a small miniature reads as belonging
   to nothing. **A leader is a feature, not a bug fix** — Peterson et al. (2009) on labels that are
   ambiguously placed.
2. **The `MESHES` tab's crossing badge leaders** — `no_two_leaders_cross`, red since 2026-09-03,
   in the MESHES door, and **not** the stage's labels. A separate mechanism in a separate tab.
3. **The other eighteen mis-slotted refusals** — carried from 2026-10-20-02, and the mechanism is
   now proven.
4. **A resumable flood with a real cursor** — AIP-158's shape, bound to the piece, the map and the
   region.
5. **Renaming a kit's namespace** — 744 mentions across 4 files, and no multi-file atomicity.

## 2026-10-21-02: the editor has a third door — an agent, and a file edited on disk

- **Change:** **three** bugs fixed and one enhancement shipped. **(1)** The badge overlay's layout
  score counted box overlaps and not leader crossings, so a crossing layout scored zero and beat
  a clean one — `no_two_leaders_cross` has been red since 2026-09-03 and is green now. **(2)**
  Twelve refusals were filed in `Status::note`, the slot a later receipt erases; they are
  `Status::problem`s now. **(3)** The new file watcher took `ResMut<Project>` and
  `ResMut<OpenMap>` **every frame**, and Bevy's conflict resolution deferred the badge layout past
  the frame — a flake the crate's own resource ratchet caught. **(4)** The editor has a **third
  door**: an **operation registry** that a
  key handler and an agent both dispatch, a **BRP bridge** of four typed commands, an **MCP
  server** (a new crate, `bevy_emerge_mcp`, on the official `rmcp` SDK) with four tools, an
  **append-only operation log**, and a **file watcher** that re-reads the project's own data files
  every 250 ms and adopts them only if they parse.

- **Why it ranked first:** the brief's focus area is *"configuration based editing allowing
  immediate feedback to the user by adjusting game editor through BRP calls"*, and **neither half
  of it existed**. Measured by reading the code: the editor read its project **once**, at open
  (`Project::open`, `project.rs:165`); the only on-demand re-read was `reload_vocab`; the only
  mtime poll in the crate watched **meshes**, not data files. **A data file edited on disk was
  invisible until a restart** — so the editor was not configuration-driven at all, because the
  configuration was read once and frozen. And an **agent** could not reach it: every verb was a
  key handler, and the only door for something without a keyboard was synthesising keystrokes —
  which the brief forbids outright (*"Never drive the UI by clicking"*).

- **Evidence:**
  - **B-001:** the score, measured on MESHES at 1680×1050 with the detail pane populated —
    **comfortable: 1 box overlap, 0 crossings, score 1. Compact: 0 box overlaps, 3 crossing
    pairs, score 0.** The compact pass won and shipped the crossings. And the lane search could
    not have saved it: an exhaustive search of all 8⁶ = 262,144 lane assignments (299,592 nodes)
    found **no** crossing-free assignment for the compact geometry, and the old fallback produced
    **eleven** crossing pairs. The fix counts crossings and makes one crossing outrank every
    overlap; the fallback now returns the fewest-crossings assignment it can find.
  - **B-002:** a survey of all 169 `status.note` sites — **12 clean refusals and 3 split sites**
    — and the rule is the `Status` type's own doc. Two of the twelve were written to *both*
    slots, so two rows said one thing and the one in the note was the one that got erased.
  - **The enhancement, end to end:** an agent lists the operations, previews one (and **nothing
    changes** — the map, the file and the log are all as they were), applies one (and **the file is
    written**), and the log says the **source** was an agent. A misspelled field is **refused**,
    not ignored — a silently dropped field is the worst outcome an agent can have, because it
    believes it changed something it did not.
  - **The two paths, measured:** the **agent's** path is **0.6–1.1 ms**; the **file's** path is
    **125–222 ms**, of which 250 ms is the poll's period. The target is about a second, and both
    are asserted against it rather than printed.
  - **The MCP server, driven as an outside agent would:** the integration test starts it as a
    subprocess and speaks MCP over stdio, reads **only the descriptions**, and calls a tool. It
    checks the annotations are there, the schema is there, and the description is long enough to
    say what the tool does.

- **Sources:** the MCP specification and `rmcp` 3.3.0's own source for the tool surface
  (`ToolAnnotations` at `src/model/tool.rs:54`, `CallToolResult::structured` at `src/model.rs:4000`,
  the `#[tool]` macro at `rmcp-macros-3.3.0/src/tool.rs`); the crate's own `Status` contract for
  the refusal rule; Bekos, Kaufmann, Symvonis & Wolff (2007) for the order-preservation argument
  the badge layout depends on; and the previous runs' records of the two bugs.

- **Test / perf:** `cargo test --workspace` → **1,320 pass, 0 fail** — all green for the first time
  since 2026-09-03. **Five new test files, 24 new tests**, and **one existing test changed**: the
  candidate-import test now reads its refusal from `status.problem_text()` rather than
  `status.note_text()`, because B-002 moved that refusal into the problem slot where a refusal
  belongs. **Its assertions are unchanged**, and no test was weakened or skipped.
  Clippy: **zero findings in any file this run touched** (the workspace's pre-existing findings
  are in other files and unchanged). `cargo fmt`: no new diff in the new files. **Frame time
  unchanged** — the watcher runs on a 250 ms tick and reads four to six files' *metadata*, not
  their contents.

- **Verdicts:** **Function: GREAT** — every state of the new path is captured: discovery,
  preview, apply, refusal, the misspelled field, the log's own record, and both latencies.
  **Fresh eyes: GREAT** — a cold start finds the tools by asking the server what it has, and a
  file edited in a text editor reaches the editor **without a restart**.

- **Commits:** `d9ba816` (the badge layout score) and `8d4ad69` (the twelve refusals), plus the
  enhancement's own commit. **No PR**, for the same reason as the previous seven runs: this
  repository has never had one (`gh api repos/Ladvien/bevy_emerge/pulls` returns **0**), every
  commit goes straight to `main`, and `gh pr create` refuses with *"head branch `main` is the same
  as base branch `main`"*.

- **Run:** `docs/loop/iterations/2026-10-21-mcp-bridge-8d4ad69.md` — the report, with the
  measurements and the test diffs.

Top 5 remaining:

1. **The 128 unmigrated actions** — the allowlist is in place, with the ratchet that stops it
   growing. **Migrate the second workflow** (the placement, or the fill) and the slice is cheap:
   the registry, the log, the watcher and the bridge are all proven.
2. **The operation log has a single-writer assumption** — the MCP server and the editor are
   separate processes, and both could append and claim the same sequence number. An advisory file
   lock would fix it, and it is the one thing in the new code that could lose an agent's record.
3. **A library or policy change re-opens the whole project** — only `Project::open` knows how to
   merge every bound kit, so the watcher re-opens the project for a `library.ron` change. A
   per-kit re-read would be better.
4. **`bevy_emerge_mcp` is unpublished** — the pre-publish gate (`semver-checks`, `package --list`,
   the dry run) has not been run, and the brief says not to publish from a red gate.
5. **Enclosure wish and weight are still hard-coded** in the editor (`ENCLOSURE_WISH = 0.25`,
   `ENCLOSURE_WEIGHT = 200`), so an author who wants more enclosed rooms still edits Rust — and
   the brief's rule is that a value somebody would tune belongs in data.
