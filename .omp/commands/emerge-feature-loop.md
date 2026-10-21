---
description: One iteration of the bevy_emerge usability loop
argument-hint: "[optional focus, e.g. 'compositions']"
---

# bevy_emerge: Biggest Usability Win

This is one iteration of a long loop on bevy_emerge, a Bevy 0.20 editor
for authoring game worlds (maps, tile kits, compositions, WFC, a VLM
labeller, an agent-drivable UI). The editor is user-facing: a game
developer sits in front of it. The intent is to have a truly expressive
game editor, where the human describes, and the agent drives. Find the
single most important usability improvement, build it, prove it by looking
at it, ship it. This should value aesthetics, performance, and usability.
Please start with searching for and addressing any bugs, then move into
enhancements. After all bugs and a single enhancement are complete, please
stop.

$ARGUMENTS

Usability means: a developer discovers it without help, does the task into
fewer steps, understands what the editor is telling them, recovers from
mistakes, and trusts the result. And! An agent can effectively execute
user commands with clarity. I.e., the editor's MCP is above par and
commands are composable.

## Directory (every agent uses exactly this)
```
docs/loop/
  log.md   needs-decision/
  runs/YYYY-MM-DD-NN/
    research.md  plan.md  perf.md  review.md
    evidence/ (before/, after/, crops/)   video/ (mp4)
```

## 1. Orient
Read `docs/loop/log.md`, `docs/` reviews and UI audits
(`crates/bevy_emerge_mapper/docs/ui_audit.md`), the crate
CLAUDE.md/AGENTS.md, `git log`, open issues, and `AGENTS_CHAT.md`.
Confirm the baseline builds and tests green, or record exactly what is
already red.

## 2. Walk the editor as a new developer
Evidence first, opinions never. Drive the editor via BRP and do real
authoring tasks on Sanctuary's project: open a door, build something
repetitive, import and label a mesh, edit a composition, undo, save,
reload. Treat every screen as a first-time user would:
- Screenshot every state at full frame, then crop and zoom 2x-4x
  (OpenCV/Pillow) on panels, badges, labels, status text, handles,
  ghosts and error messages.
- Count steps and keystrokes per task. Note every dead end, refusal,
  unclear message, hidden control, crash and surprise.
- Check the visuals: hierarchy, contrast, alignment, spacing, clutter,
  and whether state (selected, disabled, loading, failed) is obvious.
Known candidates: no way to stamp a composition for repeated
placements (282 hand-placed fence pieces), missing operations (kit and
project rename, policy add, bash authoring), the Map-door crash, label
badge crossings, status text with stray spaces, probe reporting a down
endpoint as ready.

Rank by developer impact. One sentence on why the pick beats the
runner-up. Carve oversized items to the most valuable slice. You may
invent improvements that fit the editor's design; justify the fit.

## 3. Research first
Use home-still before any code: level-editor and tool UX, direct
manipulation, discoverability, error-message design, feedback and
affordances, prefab and instancing models. Fill gaps with
`paper_search` and `paper_download`. Write `research.md` (sources,
technique, what great looks like), then `plan.md` (design, touched
modules, tests). No research, no build.

## 4. Build within the project's rules
- `bevy_emerge_core` stays engine-free; `engine_free.rs` must pass.
- A new verb adds one row to `BINDINGS` in `keys.rs`, nowhere else.
- Use design-system tokens only; the chrome census ratchets must pass.
- One path per feature: refusal over degradation, no fallbacks, no
  stubs. No `unwrap`/`expect`/`panic!` outside tests.
- Schema changes bump versions, never silently migrate. Preserve comments
  (`ron_surgery`); never reformat hand-authored RON wholesale.
- Do not grow `placement/`. A structural decision (retiring the runtime
  half, deleting `placement/`) goes to `needs-decision/` with options and
  a recommendation. Do not decide it yourself.
- The game consumes core by path: list every consumer callsite your API
  change breaks, and fix Sanctuary's side only as its own uncommitted
  change, noted in `AGENTS_CHAT.md`.
- Mind units: maps are metres, the game's constants are feet.

## 5. Test
Write the tests that prove the feature, including the untested areas it
touches. Run `cargo test --workspace`, clippy `-D warnings`, `cargo fmt
--check`, `cargo doc --no-deps`, the headless boot against Sanctuary's
project (`cargo run -q -p bevy_emerge_mapper --example boot_headless --
<game-root>`), and Sanctuary's own suite. Metamorphic or round-trip tests
beat mocks. Record timings in `perf.md`.

**A test that cannot fail is not a test.** If the harness cannot measure
the thing (no camera, no window, no device), say so in the test's own
failure message rather than letting it pass vacuously — and find a level
at which it *can* be measured. A unit test on a pure function beats a
screenshot assertion, and a screenshot beats nothing.

**Pin the *conversion*, not the comparison.** When a defect is a
disagreement between two coordinate spaces, a test that compares the
spaces to each other will pass while the screen is wrong. Measure one
space against a known value, and make the code have exactly one place
that converts.

## 6. Visual review, three passes
Judge the images yourself. Drive the editor via BRP and capture
screenshots and mp4s (ffmpeg). Before each verdict write the critique of
what is wrong. Verdicts: GREAT, NOT YET, FAIL; only GREAT passes. Log all
three in `review.md`.
1. **Function:** every state of the feature at full frame. Does it work
   and read?
2. **Zoom:** crop and upscale 2x-4x (OpenCV/Pillow) on panels, badges,
   labels, handles and ghosts. Alignment, contrast, hierarchy, beauty.
3. **Fresh eyes:** re-capture from scratch and judge as a developer who
   has never seen the editor. Can I discover it, do it, undo it?

Any fix returns you to the earliest pass it could affect. After 5 failed
rounds, log the blocker and revert.

## 7. Ship, in this order
Update docs and the changelog, correct any stale doc you touched, commit,
push, open a PR, merge to main with `gh pr merge --merge` (never
`--delete-branch`), pull main locally with `git merge --ff-only`. Never
force-push, `git reset --hard` or delete without the owner's express
permission. Never bump a version or publish; that is the owner's call.

## 8. Log
Append to `log.md`: change, evidence, why it ranked first, sources,
test/perf results, verdicts, commit, and the top 5 remaining items.

## Rules
- **Bugs first, then one enhancement.** Search for and fix the bugs you
  can find, then do a single enhancement. Stop after that.
- One feature per iteration: researched, built, tested, three GREEN
  reviews, shipped, logged.
- If blocked, log it and pick the next item.
