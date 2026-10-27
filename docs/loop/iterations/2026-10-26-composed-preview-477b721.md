# 2026-10-26 fill: a composed proposal is drawn piece by piece (E-061)

Focus: fill · Commits: `08882bd..477b721` · Machine: laptop (`mac_air`) · Outcome: shipped

## Summary

B-046 made `G` draw every proposed piece as a translucent ghost so the author sees a layout before
`Enter` keeps it. A **composed** proposal (the platform-modified `G`,
`layout_propose {source: composed}`) lays stamps rather than placements, and that arm of
`sync_proposal_ghosts` was a comment saying drawing members would duplicate
`composition::expand`'s placement rules. The author was asked to keep a layout they could not see —
the same defect one source over. The fix draws the members through `composition::expand` and
`stamped_heights` against the map `layout_accept`'s keep lands in, so the ghosts *are* the rows
`Enter` commits rather than a second expansion.

## Baseline

Gate: green before (`cargo build --workspace` clean, `clippy -D warnings` clean after the change).
`accept.py --source assets` **10/10** after; Sanctuary is not on this machine
(`/Users/ladvien/sanctuary` absent), so 10/10 — not the laptop's 11/11 — is the comparable figure,
and the previous *big* run was also 10/10. Load 5.58 at start, 131 GB free disk, 10 cores.

## Bugs fixed

None. No open bug reproduced this pass: B-018 and B-030 are both judged 1-of-3 (workaround exists,
not common) and B-021 is explicitly not reproducible; none reached must-fix.

## Enhancement: a composed proposal is drawn piece by piece

Goal: an author pressing the composed-generate key sees the pieces, not a cell outline and a count.

- **Files:** `crates/bevy_emerge_mapper/src/editor.rs` (`sync_proposal_ghosts`),
  `crates/bevy_emerge_mapper/tests/a_composed_proposal_draws_its_members.rs` (new).
- **The landing map, not a guess.** `landed` is the open map with its unpinned stamps retained away
  and the proposal's laid on — exactly what `layout_accept`'s keep removes and extends. An expansion
  against anything else would preview a map nobody gets.
- **No new machinery.** `stamped_heights` is the seating `redraw_stamps` and `drive_stamp_ghost`
  already share; `spawn_piece` + `(ProposalGhost, Ghost)` is B-046's own path. The diff is one arm.
- **Refusals stay silent on screen and loud in the status row**, matching the placement arm: a
  proposal whose stamps do not expand is one the keep would refuse too, and `layout_accept` says why.
- **Proof:** `a_composed_proposal_draws_its_members.rs` computes the expansion the keep would
  produce and asserts one ghost per expanded row; its fixture's compositions carry two members each,
  so 4 rows vs 2 stamps distinguishes the fix from "one ghost per stamp". At the base commit in a
  throwaway worktree the same test fails on `ghosts == 0`. The second test asserts `Esc` leaves no
  ghosts behind.

## Borrowed from Mixar

None.

## Performance

Not measured: this is a preview-time spawn on a keypress, and `perf.py` times a live editor.

## Acceptance

`reached 10/10` after (all of tools, make_map, open_map, place, propose, still, accept, save,
present, verify); first failure: none. No `import` step on this source, as the command states.

## Review

Reviewer not spawned — the pass reached its commit checkpoint first; the five questions are mine.

1. **What breaks if the landing map differs from the keep's?** Nothing in this diff: `retain(|s|
   s.owned)` + `extend(proposal.stamps)` is literally the keep's `unstamped` filter and its
   `extend`. Both read `owned`, so a pinned stamp survives in both.
2. **Is a ghost ever left behind?** No: `sync_proposal_ghosts` despawns every `ProposalGhost` before
   spawning, and the key includes the map origin, so a moved origin re-draws.
3. **Is `stamped_heights` given the right map?** Yes — `landed`, and it resolves the expansion
   against that map's own placements, which is what makes a member seated on a hand-placed table draw
   at the table's height.
4. **Does this change what `Enter` does?** No. Only `sync_proposal_ghosts` changed; `layout_accept`
   and `layout_propose` are untouched.
5. **Does the ghost count mean what the test says?** Yes on this fixture: two bounded compositions of
   two members, so the expansion is 4 rows and the proposal 2 stamps — the test fails a
   one-ghost-per-stamp implementation.

## Test diffs

New file only; no existing test changed. `a_composed_proposal_draws_its_members.rs` was written
failing-before and its fixture is its own (`e061-composed-drawn`, `e061-composed-discard`).

## Release

No release: E-006 open.

## Next focus

E-060 (`mount_options` offers the two-way mount for the first surface class only) or E-059-class
readout work; E-061 is closed.
