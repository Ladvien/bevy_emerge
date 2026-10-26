# 2026-10-26 body-fields: a tool's request body names every field the editor has (B-052)

Focus: parity — a tool that accepts a parameter and silently drops it · Commits: `fda4452` · Outcome: shipped (no behaviour change)

## Summary

The MCP server is a bridge. Each tool takes typed parameters and forwards them to the editor over BRP
by **building the body by hand** — `json!({"operation_id": p.operation_id, …})`. That body is a second
copy of a struct that lives in `bevy_emerge_mapper::ops_brp`, and the server crate cannot depend on the
mapper (it is the bridge's client), so nothing could see the two drift.

**They drifted, and it took a live run to find.** E-084 added `turn` to both `UndoParams` structs. Serde
accepted it at the server, the field was never put on the wire, and the editor answered *"unknown field
`turn`"* — while every deterministic check passed, because the mapper's tests call
`emerge/operation_undo` directly and never cross the server.

This iteration makes that class of defect deterministic: two source-text ratchets, in the shape
`docs_are_current` and `docs::tool_table_and_count` already use.

**It changes no behaviour.** All six forwarding bodies agree with their structs today.

## Baseline

Gate: green. `cargo clippy --all-targets -- -D warnings` clean; `cargo test --workspace` green across **139 test binaries, 0 failures**, with free disk holding at 15 GB for the whole run.
Laptop load 3.6–6.6.

**Disk, measured rather than assumed.** Free space was 14 GB against the loop's 20 GB floor, and last
iteration had cleared cargo's incremental cache, so this time I checked whether the work fits before
asking: a relink of `bevy_emerge_mapper` took 35.9 s and left **15 GB** (it freed space), and a test
binary took 2.1 s with no change. The gate runs. Nothing was deleted this iteration.

| Measure | Before | After |
|---|---|---|
| Frame time (median) | 21.2 ms | not re-measured — see Performance |
| Key latency p95 | 61.5 ms | — |
| Edit-to-visible, file | 182.4 ms | — |
| Edit-to-visible, MCP | 38.6 ms | — |
| `mcp_call_ms` | 470.0 ms | — |
| `accept.py` reached | 11 / 11 | 11 / 11 |

## Bugs fixed

### B-052: the MCP server builds tool bodies by hand, so a field the editor's params grow is dropped silently

Must-fix: blocks n, common n, no workaround **y** — a tool whose parameter is silently discarded is not
a wrong answer, it is no answer, and there is no way for a caller to tell.

**Root cause: two copies of one struct.** The tool's `*Params` (in the server crate, which must not
depend on the mapper) and the editor's `*Params` (in `ops_brp`) are kept in step by whoever writes the
`json!` body. Nothing compared them. E-084's live run is the measurement: `turn: true` reached the
editor as `unknown field turn`, was refused, and the answer read like a one-change undo.

**Fix:** two tests in `crates/bevy_emerge_mcp/tests/a_tools_body_names_every_field.rs`.

- `every_forwarded_body_names_its_editors_fields` — for each of six forwarding tools, the keys its
  `json!` body writes must equal the editor struct's field names, **in both directions**. Omitted is a
  parameter accepted and discarded; extra is a `deny_unknown_fields` refusal to a call the schema called
  valid.
- `every_forwarded_brp_method_is_checked` — every `self.editor.call` that builds a body must be in the
  table or in a named list of methods that forward nothing. **This is the half that catches the next
  tool**, which is how the first drift happened.

Regression check, and the proof it is one: removing `turn` from the undo body while it stays in the
struct —

```
`emerge/operation_undo`'s hand-built body and `UndoParams` disagree.
  left:  ["dry_run", "operation_id"]
  right: ["dry_run", "operation_id", "turn"]
```

| Before | After |
|---|---|
| the shipped tool dropped `turn`, found only by a live run | the same drift fails a deterministic check |

No stills: nothing on screen is involved, and a screenshot of a refused tool answer would not show
what the test shows.

## Enhancement

None. This iteration is one bug fix and no enhancement: the loop allows "fixes only", and the fix is
the whole of what B-052 asks. The next enhancement candidate (E-068's session panel, the catalogue's
next open person/agent beat) is untouched.

## Borrowed from Mixar

None.

## Performance

Not re-measured after the change, and it cannot have moved: two tests that read source text at compile
time and run in 0.00 s are not on any editor path. The before-run is recorded above.

## Acceptance

`reached 11/11 → 11/11`; no first failure.

## Visual proof

None, for the same reason as the report's own summary: this changes no surface. The evidence is the
reverted-body failure above.

## Review

**The reviewer could not run, for the fifth iteration running** — the `reviewer` subagent dies on a
cloud `HTTP 429` (`You reached your Pro 5-hour limit`) and takes no model override. The review below
is mine.

| Question | Finding |
|---|---|
| **1. Does the ratchet actually catch B-052?** | Yes, and that is the only reason to trust it: removing `turn` from the body fails with the exact two lists, quoted above. A ratchet nobody has seen fail is a comment. |
| **2. Does it catch the *next* tool?** | That is `every_forwarded_brp_method_is_checked`, and it is the half that matters — the first drift happened because a new body was added and nothing was watching the list. It scans `self.editor.call("emerge/…")` and demands each be in the table or in a named not-forwarded list. |
| **3. Can the parser be wrong in a way that passes?** | It can. It reads text, not Rust: a body written as `json!({ ...p.fields })` (a spread), or keys built by a `format!`, would not be seen as keys. Both shapes are absent from the crate today and neither would compile into a silently-dropped field without also failing the tool's own tests — but this is a **text** ratchet and the report says so rather than implying it parses. |
| **4. Are the six bodies really equivalent today?** | Yes — the test passes with all six, so this iteration changes no behaviour. That is stated rather than left to be inferred from a green run. |
| **5. Is there a seventh body nobody listed?** | The second test answers this by construction: it walks the source for `self.editor.call("emerge/` and fails on anything unaccounted for. It passes, so the list is complete as of this commit. |

## Test diffs

`crates/bevy_emerge_mcp/tests/a_tools_body_names_every_field.rs`, new, two tests. No existing test or
assertion changed anywhere.

| Test | Before |
|---|---|
| `every_forwarded_body_names_its_editors_fields` | **new**; fails on a body written without `turn` (proved) |
| `every_forwarded_brp_method_is_checked` | **new**; passes today, and is what catches the next tool |

Two false starts of mine, both recorded because they are the same lesson twice: the first version read
the files with `std::fs::read_to_string` on paths relative to the test file, which resolve against the
*process* and failed — `include_str!` is what `docs_are_current` uses for exactly this; and the first
key parser tracked "am I at a key start", which mis-read `"operation": p.operation` as a key named
` p.operation`. Both were caught by the test failing on its own first run, which is the argument for
running it before believing it.

## Gate

- `cargo clippy --all-targets -- -D warnings` — clean.
- `cargo test --workspace -- --test-threads=4` — green: **139 test binaries, 0 failures**, and the disk stayed at 15 GB throughout, which is the measurement that says the gate fits on this machine now.
- `cargo test -p bevy_emerge_mcp --test a_tools_body_names_every_field` — 2 passed.
- `python3 tools/loop/accept.py` — 11/11 before and after.
- `python3 tools/loop/backlog.py check` — consistent.

## Release

Version: none · Channel: **not published**. E-006 stands: no crate here has a published baseline. Two
tests were added; no published code changed.

## Deferred / open

- **E-083** — Sanctuary's 16 compositions are all `Anchored`, so the composed source and E-080's
  variety measurement cannot run on the only real project in this repo.
- **E-082** — `rules_set` cannot check descriptor ids against a library it does not have.
- **E-068** — an agent's session has no panel; the catalogue's next open person/agent beat.
- **Disk** — 15 GB free. The gate fits at that level, measured twice this iteration; the next
  iteration should re-measure rather than assume.

## Next focus

E-068, the session panel: the loop can now group a turn, undo it from either door, and measure what a
generation did — but **the person cannot see the agent's steps as they happen**, which is the one
Mixar beat the brief names that is still open. It is a surface, so it needs a sequence and stills, and
it is the last gap between "the agent drove it" and "the person watched it".
