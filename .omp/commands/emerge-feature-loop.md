---
description: One iteration of the bevy_emerge most-useful-feature loop
argument-hint: "[optional focus, e.g. 'compositions']"
---

# bevy_emerge: Next Most Usable Feature

You are one iteration of a long loop on bevy_emerge, an editor and
engine-free core for authoring game worlds in Bevy 0.20 (maps, tile kits,
compositions, wave-function-collapse, a VLM labeller, an agent-drivable
UI). Find the single feature that would most help a game developer use
it, build it, prove it, ship it. Then stop. $ARGUMENTS

"Most usable" means: an author gets a real project built faster, with
fewer sharp edges, and trusts the result.

## Directory (every agent uses exactly this)
```
docs/loop/
  log.md   needs-decision/
  runs/YYYY-MM-DD-NN/
    research.md  plan.md  perf.md  review.md
    evidence/ (png, crops)   video/ (mp4)
```

## 1. Orient
Read `docs/loop/log.md`, `docs/` technical reviews, the crate
CLAUDE.md/AGENTS.md files, `git log`, open issues, and consumer notes
(`AGENTS_CHAT.md`). Confirm the baseline is green before touching code.

## 2. Find the real friction
Gather evidence, not opinions. Use a real project (Sanctuary's) and do
actual authoring tasks in the editor: build something repetitive, edit a
composition, import a mesh, label it, undo, save, reload. Count
placements, steps and keystrokes. Note every refusal, crash, dead end
and surprise. Known candidates from the last review: stamping a
composition for repeated placements (282 hand-placed fence pieces),
editor operations that do not exist (kit/project rename, policy
add, bash authoring), tokens with no reader, untested fill patterns.
Rank by developer impact; one sentence on why the pick beats the
runner-up. Carve oversized items into the most valuable slice. You may
invent features that fit the editor's design; justify the fit.

## 3. Research first
Use home-still before any code: procedural placement and WFC, level
editor UX, prefab and instancing models, constraint authoring, direct
manipulation and discoverability, plus game-tool design write-ups. Fill
gaps with paper_search and paper_download. Write `research.md`
(sources, technique, what great looks like), then `plan.md` (design,
touched modules, tests, schema impact). No research, no build.

## 4. Build within the project's rules
- `bevy_emerge_core` stays engine-free; `engine_free.rs` must pass.
- A new verb adds one row to `BINDINGS` in `keys.rs`, nowhere else.
- Use the design system tokens; the chrome census ratchets must pass.
- One path per feature: refusal over degradation, no fallbacks, no
  legacy shims, no stubs. No `unwrap`/`expect`/`panic!` outside tests.
- Schema changes bump versions; files are refused on mismatch, never
  silently migrated. Preserve comments (`ron_surgery`); never
  reformat hand-authored RON wholesale.
- Do not grow `placement/`. A structural decision (retiring the runtime
  half, deleting `placement/`) goes to `needs-decision/` with options
  and a recommendation. Do not decide it yourself.
- The game consumes core by path: list every consumer callsite your API
  change breaks, and fix Sanctuary's side only as its own uncommitted
  change, noted in `AGENTS_CHAT.md`.
- Mind units: maps are metres, the game's constants are feet.

## 5. Test
Write the tests that prove the feature, including the untested areas it
touches. Run `cargo test --workspace`, clippy `-D warnings`, `cargo fmt
--check`, `cargo doc --no-deps`, the headless boot against Sanctuary's
project, and Sanctuary's own suite. Metamorphic or round-trip tests beat
mocks. Record timings in `perf.md`.

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
push, open a PR, merge to main with `gh pr merge --merge` (never `--delete-branch`), pull main locally with `git merge --ff-only`. Never force-push, `git reset --hard` or delete without the owner's express permission. Never bump a version
or publish; that is the owner's call.

## 8. Log
Append to `log.md`: change, evidence, why it ranked first, sources,
test/perf results, verdicts, commit, and the top 5 remaining items.

## Rules
- One feature per iteration: researched, built, tested, three GREEN
  reviews, shipped, logged.
- If blocked, log it and pick the next item.
