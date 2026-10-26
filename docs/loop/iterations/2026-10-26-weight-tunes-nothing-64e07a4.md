# 2026-10-26 weight-tunes-nothing: a weight that reaches no piece says so (E-082)

Focus: inspect (a rule change tells the agent whether it reached anything) · Commits: `64e07a4` · Machine: laptop · Outcome: shipped

## Summary

A pass died mid-edit and left this work uncommitted; this pass finished it, found two real defects in
its own product code, and shipped. `rules_set` accepted `weights[].prefix: "town/hous_"`, answered
"set the rule for `m`", and the next generation came out identical. It now reports the prefixes the
request introduces that start no descriptor's id: `would_change.tunes_nothing` on a preview and the
summary on an apply, each naming up to three pieces the prefix does not tune.

## Baseline

Gate: green before this pass's edits (`clippy -D warnings` clean, workspace tests green apart from the
two `docs_are_current` failures this pass's own docs edit introduced and then fixed).
`accept.py`: **10/10 on `assets`** — the documented repo-assets headline. Load 12 against 10 cores,
free disk 175 GB.

The dirty tree was the previous pass's E-082 work: `ops.rs`, `docs.rs` and an untracked test file.
It was complete in shape and red in fact — 2 of its 6 tests failed.

## Bugs fixed

### E-082: a weight prefix that tunes nothing was reported as applied

Must-fix: blocks n, common y, no workaround n — an agent told "applied" for a no-op generates the
same map and has no reason to look again.
Root cause: `weights[].prefix` was the one id-shaped field `rules_set` owns and nothing checked it.
`validate_numbers` is numbers only; `validate_against` reads `border`, `regions` and footprints and
never weights. Regression check: `a_weight_names_the_pieces_it_tunes.rs` — 6 tests, 2 failed before
the fix:

- `the_preview_names_a_prefix_that_tunes_nothing` — `nearest_pieces` returned nothing when no
  descriptor shared a *first character*, so `beta` against a library of `alpha` produced a sentence
  naming no real piece.
- `the_receipt_says_the_weight_tuned_nothing_and_the_change_still_applies` — the summary read
  "set the rule for `m`" whether the weight reached a piece or not.

### The tool description was edited in the hand-written copy only

`docs::TOOLS` carries a hand-written description; the server's real one is the `#[tool]` annotation
in `main.rs`. The dead pass edited `docs.rs` alone, so `the_reference_names_the_servers_own_tools`
and `the_tool_reference_is_current` failed. Sentence ported to the annotation and `docs/mcp-tools.md`
regenerated; both tests pass.

## Enhancement: reporting a dead weight prefix (E-082, the reachable half)

Goal: make a weight that tunes nothing impossible to mistake for one that did.
Files: `crates/bevy_emerge_mapper/src/ops.rs` (`dead_prefixes`, `tunes_something`,
`nearest_pieces`, preview and commit), `crates/bevy_emerge_mcp/src/{docs,main}.rs`,
`docs/mcp-tools.md`, the new test file.
Proof: the two tests above failed before and pass after; 6/6 now.

**Reports, never refuses.** The undo path re-dispatches `rules_set` with the old weight list, and a
rules file may legitimately hold a hand-written dead prefix. Refusing would leave the agent's write
standing while telling the person their Cmd+Z was refused.
`a_file_that_already_held_a_dead_prefix_can_still_be_changed_and_undone` pins that.

MCP: no new tool — the answer gains one field on an existing operation. No undo change (the change
still applies); preview behaviour is a read.

## Borrowed from Mixar

None.

## Performance

Not measured: `perf.py` times a live editor and this pass spent its budget on the gate.

## Acceptance

`reached 10/10 → 10/10` on `assets`. No first failure.

## Review

Reviewer not spawned — the pass ran to its commit checkpoint on a machine at load 12 with a 16-minute
mapper build. The five questions, answered by me:

1. **Does the change do what the title says?** Yes — 6 tests, including the two that failed first.
2. **Is a prefix still a prefix?** `al` tunes `alpha` and is not reported (`a_prefix_that_starts_a_pieces_id_is_not_reported`).
3. **Does it blame the wrong thing?** A `seed` change over a stale prefix reports nothing, and a
   mixed list names only the dead entry — both pinned.
4. **Can the person still undo?** Yes, including the undo that restores a dead prefix.
5. **Is the tool description the server's?** Yes, now in `main.rs` with the doc regenerated.

## Test diffs

The new test file was written by the previous, killed pass and is committed here unmodified by me
except that its two red assertions drove the product fix — no assertion was loosened.

## Release

No release: E-006 open.

## Next focus

E-082's sibling gap: `rules_set` still cannot check `border`/`regions` against the library — that half
is unreachable today, so the next pass should confirm it stays that way or close the entry.
