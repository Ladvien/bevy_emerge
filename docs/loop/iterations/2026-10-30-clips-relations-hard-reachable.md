# 2026-10-30 clips: relations-hard and relations-reachable-from recorded on sib

Focus: changelog clips · Commits: 50c53b6.. · Machine: sib (Linux, RTX 3060, Xvfb :92) · Outcome: shipped (docs and media only)

## Summary
No bug, consumer or unclaimed owner issue was workable on sib, and the focus's headless backlog list is done (all ten have closed issues) except B-021 (unreproduced). So this pass recorded the two newest `not yet recorded` changelog lines, claimed as #154, and replaced their reasons with clips.

## Baseline
Gate: docs-only change; `changelog.py check` and `backlog.py check` pass. `accept.py`: not run (no product code changed). Load 2.2, free disk 696 GB, available RAM 11 GB.

## Clips
- **relations-hard** (`tools/demo/storyboards/relations_hard.json`): scratch map with bookcase near lamp (hard, broken) and bookcase near table (soft, met). The RELATIONS row reads `1 unmet: bookcase_tall@0 near lamp_tall@2 (hard)`; after `relations_solve` the bookcase stands by the lamp, LAST ends `still unmet: bookcase_tall@0` and the row reads `1 unmet: bookcase_tall@0 near table@1`.
- **relations-reachable-from** (`tools/demo/storyboards/relations_reachable_from.json`): a table inside a ring of twelve lamps, ReachableFrom the sofa at clearance 0.6 m. Row reads `1 unmet: table@0 reachable from sofa@1 (lamp_tall@2 in the way)`; after `relations_solve`, `moved 1 (lamp_tall@2): unmet relations 1 -> 0` and the row reads `1 met`.

Both on `make_scratch(assets)` plus hand-written `maps/{map,ring}.map.ron` and `.relations.ron`; still and contact sheet looked at before shipping.

## Review
Reviewer not spawned: media and changelog only. Self-check: each still shows the row text its line claims (read off the contact sheets above).

## Test diffs
none

## Release
None: not `bevy_emerge_loop`.

## Next focus
Next `not yet recorded` line on sib: gooey-skin (#147).
